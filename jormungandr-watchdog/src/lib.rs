pub mod service;
mod watchdog;

pub use jormungandr_watchdog_derive::CoreServices;
pub use service::{Service, ServiceIdentifier, ServiceState};
pub use watchdog::{
    ControlHandler, CoreServices, WatchdogBuilder, WatchdogError, WatchdogMonitor, WatchdogQuery,
};
