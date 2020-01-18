//! create a ping and a pong services that can be used for testing
//! the different operations available in term of intercom and
//! monitoring how the start and shutdown process works
//!

use async_trait::async_trait;
use jormungandr_watchdog::{
    service, CoreServices, Service, ServiceIdentifier, ServiceState, WatchdogBuilder,
};
use std::time::Duration;
use tokio::time::delay_for;

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

impl service::Intercom for PingMsg {}
impl service::Intercom for PongMsg {}

#[async_trait]
impl Service for Ping {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "ping";

    type State = service::NoState;
    type Settings = service::NoSettings;
    type Intercom = PingMsg;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        let mut pong = self.state.watchdog_query.intercom::<Pong>().await.unwrap();

        while let Some(msg) = self.state.intercom_receiver.recv().await {
            dbg!(msg);
            delay_for(Duration::from_millis(50)).await;
            if let Err(_err) = pong.send(PongMsg).await {
                pong = self.state.watchdog_query.intercom::<Pong>().await.unwrap();
                if pong.send(PongMsg).await.is_err() {
                    break;
                }
            }
        }
    }
}

#[async_trait]
impl Service for Pong {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "pong";

    type State = service::NoState;
    type Settings = service::NoSettings;
    type Intercom = PongMsg;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        let mut ping = self.state.watchdog_query.intercom::<Ping>().await.unwrap();

        ping.send(PingMsg).await.unwrap();

        while let Some(msg) = self.state.intercom_receiver.recv().await {
            dbg!(msg);
            delay_for(Duration::from_millis(50)).await;
            if let Err(_err) = ping.send(PingMsg).await {
                ping = self.state.watchdog_query.intercom::<Ping>().await.unwrap();
                if ping.send(PingMsg).await.is_err() {
                    break;
                }
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
#[tokio::test]
async fn start_shutdown_watchdog() {
    let watchdog = WatchdogBuilder::new().build(PingPongServices {
        ping: service::ServiceManager::new().await,
        pong: service::ServiceManager::new().await,
    });

    let mut controller = watchdog.control();
    tokio::spawn(async move {
        delay_for(Duration::from_millis(400)).await;
        controller.shutdown().await;
    });

    let mut controller = watchdog.control();
    controller.start("ping").await.unwrap();
    controller.start("pong").await.unwrap();

    watchdog.await
}
