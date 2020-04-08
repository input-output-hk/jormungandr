use async_trait::async_trait;
use clap::App;
use jormungandr_watchdog::{
    service, CoreServices, IntercomMsg, Service, ServiceIdentifier, ServiceState, WatchdogBuilder,
};
use tokio::{
    io::{stdin, stdout, AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    stream::StreamExt as _,
};

use tracing::level_filters::LevelFilter;

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

#[derive(CoreServices)]
struct StdEcho {
    stdin: service::ServiceManager<StdinReader>,
    stdout: service::ServiceManager<StdoutWriter>,
}

struct LoggerConfig {
    level: LevelFilter,
}

struct LoggerService {

}

#[async_trait]
impl Service for LoggerService {
    const SERVICE_IDENTIFIER: &'static str = "logger";
    type State = service::NoState;
    type Settings = LoggerConfig;
    type IntercomMsg = service::NoIntercom;

    fn prepare(service_state: ServiceState<Self>) -> Self {
        unimplemented!()
    }

    async fn start(self) {
        unimplemented!()
    }
}



fn main() {
    use tracing_subscriber::{fmt, EnvFilter};

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");

    let app = App::new("stdin_echo");
    let watchdog = WatchdogBuilder::<StdEcho>::new(app).build();

    let mut controller = watchdog.control();
    watchdog.spawn(async move {
        controller.start("stdout").await.unwrap();
        controller.start("stdin").await.unwrap();
    });

    watchdog.wait_finished();
}
