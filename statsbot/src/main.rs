use clap::{Args, Parser as ClapParser, Subcommand};
use futures::{StreamExt, stream::FusedStream};
use irc::{client::prelude::Config as IrcConfig, proto::Command, proto::Message};
use serde::Deserialize;
use thiserror::Error as ThisError;
use tokio::io::AsyncReadExt;

mod metrics;

#[derive(ThisError, Debug)]
enum Error {
    #[error("IRC error: {0}")]
    Irc(#[from] irc::error::Error),

    #[error("Failed to setup signal listener: {0}")]
    SignalListener(std::io::Error),

    #[error("Failed to read configuration file: {0}")]
    ReadConfiguration(std::io::Error),

    #[error("Failed to parse configuration: {0}")]
    ParseConfiguration(#[from] serde_json::Error),

    #[error("Failure from metrics service: {0}")]
    Metrics(#[from] metrics::Error),
}

#[derive(Deserialize, Clone, PartialEq)]
struct Configuration {
    irc: IrcConfig,
    oper_name: Option<String>,
    oper_password: Option<String>,
    /// Servers to query stats for
    servers: Vec<String>,

    /// address to listen on for the prometheus exporter
    exporter_address: String,
}

#[derive(Debug, Args)]
struct ValidateConfiguration {
    path: String,
}

impl ValidateConfiguration {
    async fn run(self) {
        match load_configuration(self.path).await {
            Ok(_) => {
                println!("Configuration looks alright!");
                std::process::exit(0);
            }
            Err(e) => {
                println!("Couldn't load configuration: {:?}", e);
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug, Args)]
struct Run {
    path: String,
}

async fn handle_reload_signal(
    config_path: &str,
    config: &mut Configuration,
    irc_process: &mut tokio::task::JoinHandle<()>,
    restart_tx: &tokio::sync::mpsc::Sender<Result<(), Error>>,
    metrics_tx: &tokio::sync::mpsc::Sender<(String, parser::StatsZ)>,
) {
    let new_config = match load_configuration(config_path.to_string()).await {
        Ok(new_config) => new_config,
        Err(e) => {
            log::error!("Couldn't load new configuration: {:?}", e);
            return;
        }
    };
    if new_config == *config {
        log::info!("Ignoring reload as the configuration hasn't changed.");
        return;
    }
    *config = new_config;
    // reload config and terminal process;
    // first try reading the config, if that fails, do nothing
    irc_process.abort();
    *irc_process = tokio::spawn(wrapper(
        restart_tx.clone(),
        metrics_tx.clone(),
        config.clone(),
    ));
    return;
}
impl Run {
    async fn run(self) -> Result<(), Error> {
        // listen for sighup and reload configuration / restart the client
        let mut reload_signal = wait_for_sighup().await?;
        let mut config: Configuration = load_configuration(self.path.clone()).await?;

        let (restart_tx, mut restart_rx) = tokio::sync::mpsc::channel(1);

        let mut metrics_service = metrics::MetricsService::default();
        let (metrics_tx, metrics_rx) = tokio::sync::mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let exporter_address = config.exporter_address.clone();
        let r_tx = restart_tx.clone();
        let service_process = tokio::spawn(async move {
            let res = metrics_service
                .run(exporter_address, metrics_rx, shutdown_rx)
                .await
                .map_err(Error::Metrics);
            let _ = r_tx.send(res);
        });
        // FIXME: add reloading/reconfiguration for the prometheus endpoint

        let mut irc_process = tokio::spawn(wrapper(
            restart_tx.clone(),
            metrics_tx.clone(),
            config.clone(),
        ));

        loop {
            #[rustfmt::skip]
            tokio::select! {
		_ = reload_signal.recv() => { handle_reload_signal(&self.path, &mut config, &mut irc_process, &restart_tx, &metrics_tx).await; },
                Some(e) = restart_rx.recv() => {
                    log::error!("Process terminated, restarting in 2 seconds: {:?}", e);
                    irc_process.abort();
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    irc_process = tokio::spawn(wrapper(restart_tx.clone(),metrics_tx.clone(), config.clone()));
                }
            }
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    ValidateConfiguration(ValidateConfiguration),
    Run(Run),
}

#[derive(ClapParser)]
struct Arguments {
    #[command(subcommand)]
    command: Commands,
}

async fn wrapper(
    sender: tokio::sync::mpsc::Sender<Result<(), Error>>,
    metrics_tx: tokio::sync::mpsc::Sender<(String, parser::StatsZ)>,
    config: Configuration,
) {
    sender.send(run(config, metrics_tx).await).await.unwrap();
}

enum CollectorState {
    NotConnected,
    Idle { next: Option<usize> },
    WaitingForData { lines: Vec<String>, server: usize },
}

async fn message_handler(
    client: &mut irc::client::Client,
    config: &Configuration,
    state: &mut CollectorState,
    message: Message,
    tx: &mut tokio::sync::mpsc::Sender<(String, parser::StatsZ)>,
) -> Result<(), Error> {
    match message.command {
        Command::Response(irc::proto::Response::RPL_ENDOFMOTD, _) => {
            if let (Some(user), Some(password)) =
                (config.oper_name.clone(), config.oper_password.clone())
            {
                log::info!("Sending oper command.");
                client.send_oper(user, password)?;
            }
            *state = CollectorState::Idle { next: None };
        }
        Command::Response(irc::proto::Response::RPL_ENDOFSTATS, _) => {
            if let CollectorState::WaitingForData { lines, server } = state {
                let (lines, server) = (lines.clone(), *server);
                *state = CollectorState::Idle {
                    next: server
                        .checked_add(1)
                        .map(|v| if v >= config.servers.len() { 0 } else { v }),
                };
                let line = lines.join("\n");
                let parsed_stats = parser::parse_stats_z(&line);
                let Some(server_name) = config.servers.get(server) else {
                    log::error!("Server name for {} gone missing?!", server);
                    return Ok(());
                };
                if let Ok(parsed_stats) = parsed_stats {
                    log::debug!("parsed: {}: {:?}", server_name, parsed_stats);
                    use simple_prometheus::SimplePrometheus;
                    let server_name = config.servers.get(server).cloned();
                    log::debug!(
                        "prometheus metrics: {}",
                        parsed_stats
                            .to_prometheus_metrics(server_name.clone())
                            .unwrap_or("".into())
                    );
                    if let Some(server_name) = server_name {
                        let _ = tx.send((server_name, parsed_stats)).await;
                    }
                }
            }
        }
        ref c @ Command::Raw(ref code, ref parts) if code == "249" => match state {
            CollectorState::WaitingForData { lines, .. } => {
                if parts.len() < 3 {
                    log::warn!(
                        "/stats z line doesn't contain at least three components (<nick> z <data>): {:?}",
                        parts
                    );
                } else {
                    lines.push(parts[2..].join(" "));
                }
            }
            _ => {
                log::warn!("Received /stats z data while not expecting it: {:?}", c);
            }
        },
        _ => {}
    }
    Ok(())
}

#[derive(Default)]
struct TickHandler {
    stuck_counter: usize,
}
impl TickHandler {
    async fn handle_tick(
        &mut self,
        client: &mut irc::client::Client,
        state: &mut CollectorState,
        config: &Configuration,
    ) -> Result<(), Error> {
        match state {
            CollectorState::Idle { next } => {
                let server_n = next.unwrap_or(0);
                let server = config.servers.get(server_n).cloned();
                let command = Command::STATS(Some("z".to_owned()), server);
                log::info!("Sending: {:?}", command);
                self.stuck_counter = 0;
                client.send(Message {
                    command,
                    tags: None,
                    prefix: None,
                })?;
                *state = CollectorState::WaitingForData {
                    lines: vec![],
                    server: server_n,
                };
            }
            CollectorState::WaitingForData { server, .. } => {
                self.stuck_counter = self.stuck_counter.checked_add(1).unwrap_or(0);
                log::warn!("Last /stats z seems to be running late?!");
                if self.stuck_counter >= 3 {
                    log::warn!(
                        "Was stuck processing server {:?}, moving on to next.",
                        config.servers.get(*server)
                    );
                    // Skip the current server and go to next.
                    *state = CollectorState::Idle {
                        next: server
                            .checked_add(1)
                            .map(|v| if v >= config.servers.len() { 0 } else { v }),
                    };
                }
            }
            _ => {}
        }
        Ok(())
    }
}

async fn run(
    config: Configuration,
    mut metrics_tx: tokio::sync::mpsc::Sender<(String, parser::StatsZ)>,
) -> Result<(), Error> {
    let mut client = irc::client::Client::from_config(config.irc.clone()).await?;
    log::info!("Connected");
    client.identify()?;

    let mut s = client.stream()?;

    let mut timer = tokio::time::interval(std::time::Duration::from_secs(5));

    let mut state = CollectorState::NotConnected;
    let mut tick_handler = TickHandler::default();
    loop {
        tokio::select! {
            Some(Ok(message)) = s.next() => message_handler(&mut client, &config, &mut state, message, &mut metrics_tx).await?,
            _ = timer.tick() => tick_handler.handle_tick(&mut client, &mut state, &config).await?,
        }
        if s.is_terminated() {
            break;
        }
    }

    Ok(())
}

async fn wait_for_sighup() -> Result<tokio::signal::unix::Signal, Error> {
    use tokio::signal::unix::{SignalKind, signal};
    signal(SignalKind::hangup()).map_err(Error::SignalListener)
}

async fn load_configuration(path: String) -> Result<Configuration, Error> {
    let mut fh = tokio::fs::File::open(path)
        .await
        .map_err(Error::ReadConfiguration)?;
    let mut data = String::new();
    fh.read_to_string(&mut data)
        .await
        .map_err(Error::ReadConfiguration)?;
    let config = serde_json::from_str(&data).map_err(Error::ParseConfiguration)?;
    Ok(config)
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Arguments::parse();

    match args.command {
        Commands::ValidateConfiguration(vc) => vc.run().await,
        Commands::Run(run) => {
            run.run().await.unwrap();
        }
    }
}
