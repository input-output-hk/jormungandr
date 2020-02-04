//! this file test mainly the watchdog properties without
//! services to add noises around.
//!

use jormungandr_watchdog::{CoreServices, WatchdogBuilder, WatchdogError};
use std::{any::Any, time::Duration};
use tokio::time::delay_for;

#[derive(CoreServices)]
struct NoServices;

/// test that the execution of the watchdog will be stopped shortly
/// after receiving the shutdown command from the controller
#[test]
fn start_shutdown_watchdog() {
    let watchdog = WatchdogBuilder::new().build::<NoServices>();
    let mut controller = watchdog.control();

    watchdog.spawn(async move {
        delay_for(Duration::from_millis(10)).await;
        controller.shutdown().await;
    });

    watchdog.wait_finished();
}

/// test that the execution of the watchdog will be stopped shortly
/// after receiving the kill command from the controller
#[test]
fn start_kill_watchdog() {
    let watchdog = WatchdogBuilder::new().build::<NoServices>();
    let mut controller = watchdog.control();

    watchdog.spawn(async move {
        delay_for(Duration::from_millis(10)).await;
        controller.kill().await;
    });

    watchdog.wait_finished();
}

/// starting an unknown service will fail and the error will
/// be appropriately reported back up to the monitor
#[test]
fn start_unknown_service() {
    let watchdog = WatchdogBuilder::new().build::<NoServices>();
    let mut controller = watchdog.control();

    watchdog.spawn(async move {
        let result = controller.start("unknown").await;
        assert_eq!(
            result,
            Err(WatchdogError::UnknownService {
                service_identifier: "unknown",
                possible_values: &[]
            })
        );

        delay_for(Duration::from_millis(10)).await;
        controller.kill().await;
    });

    watchdog.wait_finished()
}
