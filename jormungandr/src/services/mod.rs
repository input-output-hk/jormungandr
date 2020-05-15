//! This module defines all the different services available
//! in jormungandr.
//!
mod config;
mod console;
mod logger;
mod monitor;

pub use self::{
    config::{ConfigApi, ConfigService},
    console::{ConsoleApi, ConsoleService},
    logger::LoggerService,
    monitor::MonitorService,
};
use organix::{service::ServiceManager, Organix, WatchdogBuilder};
use std::sync::atomic::{AtomicI32, Ordering};

static RETURN_CODE: AtomicI32 = AtomicI32::new(1);

/// helper function to help build the different services
///
/// this function is to call in the event of an error that is not
/// recoverable from in order to exit the software successfully
pub(self) async fn fatal_error<S, E>(state: &mut organix::ServiceState<S>, error: E)
where
    E: std::error::Error + Send + 'static,
    S: organix::Service,
{
    tracing::info!(%error, "received a fatal error");

    ConsoleApi::error(&mut state.intercom_with::<ConsoleService>(), error);

    shutdown_with(state, 1).await
}

pub(self) async fn shutdown_with<S>(state: &mut organix::ServiceState<S>, code: i32)
where
    S: organix::Service,
{
    RETURN_CODE.store(code, Ordering::SeqCst);

    state.watchdog_controller().clone().shutdown().await
}

/// All services of the Jörmungandr app to be added in this field.
///
/// By default all services are going to use a _shared_ runtime
/// with `io` and `time` driver (from tokio) already enabled.
///
/// However, consider using `#[runtime(io, time)]` for the service
/// who need their own runtime defined.
#[derive(Organix)]
#[runtime(shared)]
struct JormungandrApp {
    console: ServiceManager<ConsoleService>,
    logger: ServiceManager<LoggerService>,
    /// Node's monitoring service
    ///
    /// This is responsible to boot the other services and
    /// to keep them up and running as it is necessary
    monitoring: ServiceManager<MonitorService>,
    /// Node's configuration service
    ///
    /// the configuration service can run on the shared runtime as
    /// it is supposed to be lightweight enough.
    configuration: ServiceManager<ConfigService>,
}

/// services entry point
///
/// This function will block until the end of the application runtime
pub fn entry() -> i32 {
    // build the watchdog monitor
    let watchdog = WatchdogBuilder::<JormungandrApp>::new().build();

    // the controller to spawn the initial services
    let mut controller = watchdog.control();

    watchdog.spawn(async move {
        controller
            .start::<MonitorService>()
            .await
            .expect("Cannot start the Monitoring service");
    });

    watchdog.wait_finished();

    RETURN_CODE.load(Ordering::SeqCst)
}
