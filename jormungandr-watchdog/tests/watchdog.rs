//! this file test mainly the watchdog properties without
//! services to add noises around.
//!

use jormungandr_watchdog::{
    CoreServices, ServiceIdentifier, WatchdogBuilder, WatchdogError, WatchdogQuery,
};
use std::{any::Any, time::Duration};
use tokio::time::{delay_for, timeout};

struct NoServices;
impl CoreServices for NoServices {
    fn start(
        &mut self,
        service_identifier: ServiceIdentifier,
        _watchdog_query: WatchdogQuery,
    ) -> Result<(), WatchdogError> {
        Err(WatchdogError::UnknownService {
            service_identifier,
            possible_values: &[],
        })
    }
    fn stop(&mut self, service_identifier: ServiceIdentifier) -> Result<(), WatchdogError> {
        Err(WatchdogError::UnknownService {
            service_identifier,
            possible_values: &[],
        })
    }
    fn intercoms(
        &mut self,
        service_identifier: ServiceIdentifier,
    ) -> Result<Box<dyn Any + Send>, WatchdogError> {
        Err(WatchdogError::UnknownService {
            service_identifier,
            possible_values: &[],
        })
    }
}

/// test that running the watchdog and then awaiting
/// on it will busy wait forever unless interrupted
/// (here by the timeout).
#[tokio::test]
async fn start_await_watchdog() {
    let watchdog = WatchdogBuilder::new().build(NoServices);

    let t = timeout(Duration::from_millis(20), watchdog).await;
    assert!(t.is_err());
}

/// test that the execution of the watchdog will be stopped shortly
/// after receiving the shutdown command from the controller
#[tokio::test]
async fn start_shutdown_watchdog() {
    let watchdog = WatchdogBuilder::new().build(NoServices);
    let mut controller = watchdog.control();

    tokio::spawn(async move {
        delay_for(Duration::from_millis(10)).await;
        controller.shutdown().await;
    });

    let instant = std::time::Instant::now();
    let t = timeout(Duration::from_millis(20), watchdog).await;
    assert!(t.is_ok());
    let elapsed = instant.elapsed();

    assert!(elapsed >= Duration::from_millis(10));
}

/// test that the execution of the watchdog will be stopped shortly
/// after receiving the kill command from the controller
#[tokio::test]
async fn start_kill_watchdog() {
    let watchdog = WatchdogBuilder::new().build(NoServices);
    let mut controller = watchdog.control();

    tokio::spawn(async move {
        delay_for(Duration::from_millis(10)).await;
        controller.kill().await;
    });

    let instant = std::time::Instant::now();
    let t = timeout(Duration::from_millis(20), watchdog).await;
    assert!(t.is_ok());
    let elapsed = instant.elapsed();

    assert!(elapsed >= Duration::from_millis(10));
}

/// starting an unknown service will fail and the error will
/// be appropriately reported back up to the monitor
#[tokio::test]
async fn start_unknown_service() {
    let watchdog = WatchdogBuilder::new().build(NoServices);
    let mut controller = watchdog.control();

    let result = controller.start("unknown").await;
    assert_eq!(
        result,
        Err(WatchdogError::UnknownService {
            service_identifier: "unknown",
            possible_values: &[]
        })
    );
}
