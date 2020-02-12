#[macro_use]
extern crate clap;

pub mod service;
mod watchdog;

pub use jormungandr_watchdog_derive::CoreServices;
pub use service::{Service, ServiceIdentifier, ServiceState};
pub use watchdog::{CoreServices, WatchdogBuilder, WatchdogError, WatchdogMonitor, WatchdogQuery};
