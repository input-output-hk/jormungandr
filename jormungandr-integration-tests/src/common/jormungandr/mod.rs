pub mod commands;
mod configuration_builder;
pub mod logger;
pub mod process;
pub mod starter;

pub use configuration_builder::ConfigurationBuilder;
pub use process::*;
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
