//! create a ping and a pong services that can be used for testing
//! the different operations available in term of intercom and
//! monitoring how the start and shutdown process works
//!

use async_trait::async_trait;
use clap::App;
use jormungandr_watchdog::{
    service, CoreServices, Service, ServiceIdentifier, ServiceState, WatchdogBuilder,
};
use std::time::Duration;
use tokio::time::delay_for;

const DEFAULT_ARGS: &[&str] = &[];

struct Ping {
    state: ServiceState<Self>,
}
struct Pong {
    state: ServiceState<Self>,
}

#[derive(Debug)]
struct PingMsg;
#[derive(Debug)]
struct PongMsg;

impl service::IntercomMsg for PingMsg {}
impl service::IntercomMsg for PongMsg {}

#[async_trait]
impl Service for Ping {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "ping";

    type State = service::NoState;
    type Settings = service::NoSettings;
    type IntercomMsg = PingMsg;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        let mut pong = self.state.intercom_with::<Pong>();

        while let Some(msg) = self.state.intercom_mut().recv().await {
            dbg!(msg);
            delay_for(Duration::from_millis(50)).await;
            if let Err(err) = pong.send(PongMsg).await {
                dbg!(err);
                break;
            }
        }
    }
}

#[async_trait]
impl Service for Pong {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "pong";

    type State = service::NoState;
    type Settings = service::NoSettings;
    type IntercomMsg = PongMsg;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        let mut ping = self.state.intercom_with::<Ping>();

        ping.send(PingMsg).await.unwrap();

        while let Some(msg) = self.state.intercom_mut().recv().await {
            dbg!(msg);
            delay_for(Duration::from_millis(50)).await;
            if let Err(err) = ping.send(PingMsg).await {
                dbg!(err);
                break;
            }
        }
    }
}

#[derive(CoreServices)]
struct PingPongServices {
    ping: service::ServiceManager<Ping>,
    pong: service::ServiceManager<Pong>,
}

/// test that the execution of the watchdog will be stopped shortly
/// after receiving the shutdown command from the controller
#[test]
fn ping_pong() {
    use tracing_subscriber::{fmt, EnvFilter};

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");

    let app = App::new("ping_pong");
    let watchdog = WatchdogBuilder::<PingPongServices>::new(app).build_from_safe(DEFAULT_ARGS);

    let mut controller = watchdog.control();
    watchdog.spawn(async move {
        controller.start("ping").await.unwrap();
        controller.start("pong").await.unwrap();
        delay_for(Duration::from_millis(400)).await;
        controller.shutdown().await;
    });

    watchdog.wait_finished();
}
