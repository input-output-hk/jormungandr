use async_trait::async_trait;
use clap::{App, Arg, ArgMatches};
use jormungandr_watchdog::{
    service, CoreServices, IntercomMsg, Service, ServiceIdentifier, ServiceState, Settings,
    WatchdogBuilder,
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{stdin, stdout, AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    stream::StreamExt as _,
};
use tracing_subscriber::fmt::Subscriber;

struct StdinReader {
    state: ServiceState<Self>,
}

struct StdoutWriter {
    state: ServiceState<Self>,
}

#[derive(Debug, IntercomMsg)]
struct WriteMsg(String);

#[async_trait]
impl Service for StdinReader {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "stdin";

    type State = service::NoState;
    type Settings = service::NoSettings;
    type IntercomMsg = service::NoIntercom;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        let mut stdout = self.state.intercom_with::<StdoutWriter>();
        let mut stdin = BufReader::new(stdin()).lines();

        while let Some(msg) = stdin.next().await {
            match msg {
                Err(err) => {
                    tracing::error!(%err);
                    break;
                }
                Ok(line) if line == "quit" => {
                    self.state.watchdog_controller().clone().shutdown().await;
                    break;
                }
                Ok(line) => {
                    tracing::debug!(%line, "read from stdin");
                    if let Err(err) = stdout.send(WriteMsg(line)).await {
                        tracing::error!(%err);
                        break;
                    }
                }
            }
        }
    }
}

#[async_trait]
impl Service for StdoutWriter {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "stdout";

    type State = service::NoState;
    type Settings = service::NoSettings;
    type IntercomMsg = WriteMsg;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        let mut stdout = stdout();

        while let Some(WriteMsg(msg)) = self.state.intercom_mut().recv().await {
            if let Err(err) = stdout.write_all(msg.as_bytes()).await {
                tracing::error!(%err);
                break;
            }
            stdout.write_all("\n".as_bytes()).await.unwrap();
            stdout.flush().await.unwrap();
        }
    }
}

const LOGGER_CONFIG_LOG_LEVEL: &str = "logger config log level";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(remote = "tracing::Level")]
pub enum LogLevel {
    INFO,
    WARN,
    DEBUG,
    ERROR,
    TRACE,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoggerConfig {
    #[serde(with = "LogLevel")]
    level: tracing::Level,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        LoggerConfig {
            level: tracing::Level::ERROR,
        }
    }
}

impl Settings for LoggerConfig {
    fn add_cli_args<'a, 'b>() -> Vec<Arg<'a, 'b>> {
        vec![Arg::with_name(LOGGER_CONFIG_LOG_LEVEL)
            .long("log-level")
            .takes_value(true)
            .default_value("warn")
            .possible_values(&["info", "warn", "debug", "error", "trace"])
            .env("LOG_LEVEL")
            .value_name("LOG_LEVEL")
            .help("Services log level: [info, warn, debug, error, trace]")]
    }

    fn matches_cli_args<'a>(&mut self, matches: &ArgMatches<'a>) {
        if let Some(level) = matches.value_of(LOGGER_CONFIG_LOG_LEVEL) {
            self.level = match level.to_lowercase().as_str() {
                "info" => tracing::Level::INFO,
                "warn" => tracing::Level::WARN,
                "debug" => tracing::Level::DEBUG,
                "error" => tracing::Level::ERROR,
                "trace" => tracing::Level::TRACE,
                _ => unreachable!(),
            };
        }
    }
}

struct LoggerService {
    state: ServiceState<Self>,
}

#[async_trait]
impl Service for LoggerService {
    const SERVICE_IDENTIFIER: &'static str = "logger";
    type State = service::NoState;
    type Settings = LoggerConfig;
    type IntercomMsg = service::NoIntercom;

    fn prepare(state: ServiceState<Self>) -> Self {
        let subscriber = Subscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting tracing default failed");
        LoggerService { state }
    }

    async fn start(mut self) {
        let mut settings = self.state.settings().clone();
        while let Some(cfg) = settings.updated().await {
            let subscriber = Subscriber::builder()
                .with_max_level(cfg.level.clone())
                .finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting tracing default failed");
            print!("{:?}", cfg);
        }
    }
}

#[derive(CoreServices)]
struct StdEcho {
    stdin: service::ServiceManager<StdinReader>,
    stdout: service::ServiceManager<StdoutWriter>,
    logger: service::ServiceManager<LoggerService>,
}

fn main() {
    // let subscriber = fmt::Subscriber::builder()
    //     .with_env_filter(EnvFilter::from_default_env())
    //     .finish();
    //
    // tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");

    let app = App::new("stdin_echo");
    let watchdog = WatchdogBuilder::<StdEcho>::new(app).build();

    let mut controller = watchdog.control();
    watchdog.spawn(async move {
        controller.start("stdout").await.unwrap();
        controller.start("stdin").await.unwrap();
        controller.start("logger").await.unwrap();
    });
    watchdog.wait_finished();
}
