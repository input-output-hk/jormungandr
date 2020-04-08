#[macro_use]
extern crate clap;

pub mod service;
mod watchdog;

pub use jormungandr_watchdog_derive::{CoreServices, IntercomMsg};
pub use service::{Service, ServiceIdentifier, ServiceState, Settings};
pub use watchdog::{CoreServices, WatchdogBuilder, WatchdogError, WatchdogMonitor, WatchdogQuery};
