use futures::{future::BoxFuture, lock};
use http_body_util::{Full, combinators};
use hyper::{
    Request, Response,
    body::{Bytes, Incoming},
};
use std::sync::Arc;
use thiserror::Error as ThisError;
use tokio::net::ToSocketAddrs;

use simple_prometheus::SimplePrometheus;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Failed to listen: {0}")]
    ListenError(#[from] std::io::Error),
}

#[derive(Default)]
pub struct MetricsService {
    metrics: MetricsStore,
}

type MetricsReceiver = tokio::sync::mpsc::Receiver<(String, parser::StatsZ)>;

type MetricsStore = Arc<tokio::sync::Mutex<std::collections::HashMap<String, parser::StatsZ>>>;
pub fn make_handler(
    registry: MetricsStore,
) -> impl Fn(Request<Incoming>) -> BoxFuture<'static, tokio::io::Result<Response<String>>> {
    move |request: Request<Incoming>| {
        let reg = registry.clone();
        Box::pin(async move {
            let reg = reg.lock().await;
            let mut buf = String::new();
            for (server, metrics) in reg.iter() {
                let s = metrics.to_prometheus_metrics(Some(server.clone())).unwrap();
                buf.push_str(&s);
            }
            drop(reg);
            Ok(Response::builder().body(buf).unwrap())
        })
    }
}

impl MetricsService {
    async fn handle_message(
        &mut self,
        server: String,
        metrics: parser::StatsZ,
    ) -> Result<(), Error> {
        log::trace!("Acquiring metrics lock for {}", server);
        let mut m = self.metrics.lock().await;
        log::info!("Updating metrics for {}", server);
        m.entry(server)
            .and_modify(|v| *v = metrics.clone())
            .or_insert(metrics.clone());
        drop(m);
        Ok(())
    }

    async fn run_server<Addr: ToSocketAddrs + std::fmt::Debug>(
        server_address: Addr,
        metrics: MetricsStore,
        shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<(), Error> {
        log::info!("Listening at {:?}", server_address);
        let listener = tokio::net::TcpListener::bind(server_address)
            .await
            .map_err(Error::ListenError)?;
        let server = hyper::server::conn::http1::Builder::new();
        let (shutdown_tx, graceful_shutdown_rx) = tokio::sync::broadcast::channel(1);

        // broadcast shutdown to all our inflight subscribers to gracefully shut them down
        let shutdown_pipe = tokio::spawn(async move {
            if let Ok(_) = shutdown_rx.await {
                let _ = shutdown_tx.send(());
            }
        });

        while let Ok((stream, remote)) = listener.accept().await {
            log::trace!("Accepting HTTP connection from {:?}", remote);
            let server = server.clone();
            let metrics = metrics.clone();
            let io = hyper_util::rt::TokioIo::new(stream);

            let shutdown_rx = graceful_shutdown_rx.resubscribe();
            tokio::spawn(async move {
                let mut shutdown_rx = shutdown_rx;
                let conn = server.serve_connection(
                    io,
                    hyper::service::service_fn(make_handler(metrics.clone())),
                );
                tokio::pin!(conn);
                #[rustfmt::skip]
                tokio::select! {
                    _ = conn.as_mut() => {},
		    _ = shutdown_rx.recv() => {
			conn.as_mut().graceful_shutdown();
		    }
                }
            });
        }
        shutdown_pipe.abort();

        Ok(())
    }

    pub async fn run<Addr: ToSocketAddrs + std::fmt::Debug + Send + 'static>(
        &mut self,
        address: Addr,
        mut rx: MetricsReceiver,
        shutdown_signal: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<(), Error> {
        let (tx, mut server_died_signal) = tokio::sync::oneshot::channel();
        let server = Self::run_server(address, self.metrics.clone(), shutdown_signal);
        let task = tokio::spawn(async move {
            if let Err(e) = server.await {
                log::error!("Server died: {:?}", e);
            }
            let _ = tx.send(());
        });

        loop {
            #[rustfmt::skip]
            tokio::select! {
                Some((server, msg)) = rx.recv() => self.handle_message(server,msg).await?,
                _ = &mut server_died_signal=> { log::error!("Webserver exited"); break; }
            }
        }
        task.abort();

        Ok(())
    }
}
