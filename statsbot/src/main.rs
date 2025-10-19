use clap::{Args, Parser as ClapParser, Subcommand};
use futures::StreamExt;
use irc::{client::prelude::Config as IrcConfig, proto::Command, proto::Message};
use serde::Deserialize;
use thiserror::Error as ThisError;
use tokio::io::AsyncReadExt;

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
}

#[derive(Deserialize, Clone, PartialEq)]
struct Configuration {
    irc: IrcConfig,
    oper_name: Option<String>,
    oper_password: Option<String>,
    /// Servers to query stats for
    servers: Vec<String>,
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

impl Run {
    async fn run(self) -> Result<(), Error> {
        // listen for sighup and reload configuration / restart the client
        let mut reload_signal = wait_for_sighup().await?;
        let mut config: Configuration = load_configuration(self.path.clone()).await?;
        let (restart_tx, mut restart_rx) = tokio::sync::mpsc::channel(1);

        let mut process = tokio::spawn(wrapper(restart_tx.clone(), config.clone()));

        loop {
            tokio::select! {
                _ = reload_signal.recv() => {
                    let new_config = match load_configuration(self.path.clone()).await {
                Ok(new_config) => new_config,
                Err(e) => {
                log::error!("Couldn't load new configuration: {:?}", e);
                continue;
                }
                    };
            if new_config == config {
                log::info!("Ignoring reload as the configuration hasn't changed.");
                continue;
            }
            config = new_config;
                    // reload config and terminal process;
                    // first try reading the config, if that fails, do nothing
                    process.abort();
                    process = tokio::spawn(wrapper(restart_tx.clone(), config.clone()));
                    continue;
                },
            Some(e) = restart_rx.recv() => {
            log::error!("Process terminated, restarting in 2 seconds: {:?}", e);
                    process.abort();
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    process = tokio::spawn(wrapper(restart_tx.clone(), config.clone()));
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

async fn wrapper(sender: tokio::sync::mpsc::Sender<Result<(), Error>>, config: Configuration) {
    sender.send(run(config).await).await.unwrap();
}

enum State {
    Idle { next: Option<usize> },
    WaitingForData { lines: Vec<String>, server: usize },
}
async fn run(config: Configuration) -> Result<(), Error> {
    let mut client = irc::client::Client::from_config(config.irc).await?;
    log::info!("Connected");
    client.identify()?;

    let mut s = client.stream()?;

    let mut timer = tokio::time::interval(std::time::Duration::from_secs(15));

    let mut state = State::Idle { next: None };
    let msg_handler = async |state: &mut State, message: Message| -> Result<(), Error> {
        match message.command {
            Command::Response(irc::proto::Response::RPL_ENDOFMOTD, _) => {
                if let (Some(user), Some(password)) =
                    (config.oper_name.clone(), config.oper_password.clone())
                {
                    log::info!("Sending oper command: {} {}", user, password);
                    client.send_oper(user, password)?;
                }
            }
            Command::Response(irc::proto::Response::RPL_ENDOFSTATS, _) => {
                if let State::WaitingForData { lines, server } = state {
                    let (lines, server) = (lines.clone(), *server);
                    *state = State::Idle {
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
                    log::info!("parsed: {}: {:?}", server_name, parsed_stats);
                }
            }
            ref c @ Command::Raw(ref code, ref parts) if code == "249" => match state {
                State::WaitingForData { lines, .. } => {
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
    };

    let mut stuck_counter: usize = 0;
    loop {
        #[rustfmt::skip]
        tokio::select! {
            Some(Ok(message)) = s.next() => {
		msg_handler(&mut state, message).await?;
            }
            _ = timer.tick() => {
		match state {
		    State::Idle { next } => {
			    let server_n = next.unwrap_or(0);
			    let server = config.servers.get(server_n).cloned();
			    let command = Command::STATS(Some("z".to_owned()), server);
			    log::info!("Sending: {:?}", command);
			    stuck_counter = 0;
			    client.send(Message {
				command,
				tags: None,
				prefix: None,
			    })?;
			    state = State::WaitingForData {
				lines: vec![],
				server: server_n,
			    }
		    }
		    State::WaitingForData { server, .. } => {
			stuck_counter = stuck_counter.checked_add(1).unwrap_or(0);
			log::warn!("Last /stats z seems to be running late?!");
			if stuck_counter > 3 {
			    log::warn!("Was stuck processing server {:?}, moving on to next.", config.servers.get(server));
			    // Skip the current server and go to next.
			    state = State::Idle {
				next: server.checked_add(1).map(|v|  if v >= config.servers.len() { 0 } else { v }),
			    };
			}
		    }
		}
	    }
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
