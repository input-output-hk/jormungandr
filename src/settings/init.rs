use crate::blockcfg::genesis_data::{Discrimination, InitialUTxO, LinearFee, PublicKey};
use crate::settings::command_arguments::*;
use crate::settings::logging::LogSettings;

use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    Config(serde_yaml::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Config(e) => write!(f, "config error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            Error::Config(e) => Some(e),
        }
    }
}

/// Overall Settings for node
#[derive(Debug)]
pub struct Settings {
    pub log_settings: LogSettings,

    pub initial_utxos: Vec<InitialUTxO>,

    pub bft_leaders: Vec<PublicKey>,

    pub slot_duration: std::time::Duration,

    pub epoch_stability_depth: usize,

    pub blockchain_start: std::time::SystemTime,

    pub allow_account_creation: bool,

    pub linear_fee: LinearFee,
    pub address_discrimination: Discrimination,
}

impl Settings {
    /// Load the settings
    /// - from the command arguments
    /// - from the config
    ///
    /// This function will print&exit if anything is not as it should be.
    pub fn load(
        command_line: &CommandLine,
        command_arguments: &InitArguments,
    ) -> Result<Self, Error> {
        let log_settings = generate_log_settings(&command_line);

        Ok(Settings {
            address_discrimination: command_arguments.address_discrimination,
            log_settings: log_settings,
            initial_utxos: command_arguments.initial_utxos.clone(),
            slot_duration: command_arguments.slot_duration.clone(),
            epoch_stability_depth: command_arguments.epoch_stability_depth,
            blockchain_start: std::time::SystemTime::now(),
            bft_leaders: command_arguments.bft_leaders.clone(),
            allow_account_creation: command_arguments.allow_account_creation,
            linear_fee: LinearFee {
                constant: command_arguments.linear_fee_constant,
                coefficient: command_arguments.linear_fee_coefficient,
                certificate: command_arguments.linear_fee_certificate,
            },
        })
    }
}

fn generate_log_settings(command_arguments: &CommandLine) -> LogSettings {
    let level = if command_arguments.verbose == 0 {
        0
    } else {
        command_arguments.verbose
    };
    LogSettings {
        verbosity: match level {
            0 => slog::Level::Warning,
            1 => slog::Level::Info,
            2 => slog::Level::Debug,
            _ => slog::Level::Trace,
        },
        format: command_arguments.log_format.clone(),
    }
}
