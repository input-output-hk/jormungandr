//! This module show how one can use legacy tokio future in the
//! context of async/await service provided here.

use async_trait::async_trait;
use clap::App;
use jormungandr_watchdog::{
    service, CoreServices, Service, ServiceIdentifier, ServiceState, WatchdogBuilder,
};
use std::time::Duration;
use tokio::time::delay_for;
use tokio_compat::prelude::*;

const DEFAULT_ARGS: &[&str] = &[];

struct Echo {
    state: ServiceState<Self>,
    sender: legacy_tokio::sync::mpsc::Sender<EchoMsg>,
    receiver: legacy_tokio::sync::mpsc::Receiver<EchoMsg>,
}
struct Client {
    state: ServiceState<Self>,
}

#[derive(Debug)]
struct EchoMsg(String);

#[derive(Debug)]
struct QueryLine(tokio::sync::oneshot::Sender<legacy_tokio::sync::mpsc::Sender<EchoMsg>>);

impl service::IntercomMsg for QueryLine {}

#[async_trait]
impl Service for Echo {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "echo";

    type State = service::NoState;
    type Settings = service::NoSettings;
    type IntercomMsg = QueryLine;

    fn prepare(state: ServiceState<Self>) -> Self {
        let (sender, receiver) = legacy_tokio::sync::mpsc::channel(10);

        Self {
            state,
            sender,
            receiver,
        }
    }

    async fn start(mut self) {
        use legacy_futures::stream::Stream as _;

        let future = self.receiver.for_each(|EchoMsg(msg)| {
            println!("{}", msg);
            Ok(())
        });

        self.state.spawn(async move {
            future.compat().await.unwrap();
        });

        while let Some(QueryLine(reply)) = self.state.intercom_mut().recv().await {
            reply.send(self.sender.clone()).unwrap();
        }
    }
}

#[async_trait]
impl Service for Client {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "client";

    type State = service::NoState;
    type Settings = service::NoSettings;
    type IntercomMsg = service::NoIntercom;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        use legacy_futures::sink::Sink as _;

        let mut echo = self.state.intercom_with::<Echo>();
        let (sender, receiver) = tokio::sync::oneshot::channel();

        echo.send(QueryLine(sender)).await.unwrap();

        let intercom = receiver.await.unwrap();

        intercom
            .send(EchoMsg("Hello Compat".to_owned()))
            .compat()
            .await
            .unwrap();
    }
}

#[derive(CoreServices)]
struct EchoServices {
    echo: service::ServiceManager<Echo>,
    client: service::ServiceManager<Client>,
}

/// test that the execution of the watchdog will be stopped shortly
/// after receiving the shutdown command from the controller
#[test]
fn compat() {
    use tracing_subscriber::{fmt, EnvFilter};

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");

    let app = App::new("compat");
    let watchdog = WatchdogBuilder::<EchoServices>::new(app).build_from_safe(DEFAULT_ARGS);

    let mut controller = watchdog.control();
    watchdog.spawn(async move {
        controller.start("echo").await.unwrap();
        controller.start("client").await.unwrap();
        delay_for(Duration::from_millis(1_000)).await;
        controller.shutdown().await;
    });

    watchdog.wait_finished();
}
