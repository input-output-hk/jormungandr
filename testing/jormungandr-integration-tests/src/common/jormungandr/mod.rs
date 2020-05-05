mod benchmark;
mod configuration_builder;
pub mod logger;
pub mod process;
mod rest;
pub mod starter;
pub use benchmark::storage_loading_benchmark_from_log;
pub use configuration_builder::ConfigurationBuilder;
pub use logger::{JormungandrLogger, LogEntry};
pub use process::*;
pub use rest::JormungandrRest;
pub use starter::*;

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JormungandrError {
    #[error("error in logs. Error lines: {error_lines}, logs location: {log_location}, full content:{logs}")]
    ErrorInLogs {
        logs: String,
        log_location: PathBuf,
        error_lines: String,
    },
}
