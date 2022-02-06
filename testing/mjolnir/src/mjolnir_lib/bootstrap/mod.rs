mod config;
mod scenario;

use crate::mjolnir_lib::MjolnirError;
use chain_impl_mockchain::key::Hash;
use config::{ClientLoadConfig, PassiveBootstrapLoad, ScenarioType};
use jormungandr_automation::jormungandr::grpc::JormungandrClient;
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ClientLoadCommandError {
    #[error("No scenario defined for run. Available: [duration,iteration]")]
    NoScenarioDefined,
    #[error("Client Error")]
    ClientError(#[from] MjolnirError),
}

#[derive(StructOpt, Debug)]
pub struct ClientLoadCommand {
    /// Number of threads
    #[structopt(short = "c", long = "count", default_value = "3")]
    pub count: u32,
    /// address in format:
    /// /ip4/54.193.75.55/tcp/3000
    #[structopt(short = "a", long = "address")]
    pub address: String,

    /// amount of delay [seconds] between sync attempts
    #[structopt(short = "p", long = "pace", default_value = "2")]
    pub pace: u64,

    #[structopt(short = "d", long = "storage")]
    pub initial_storage: Option<PathBuf>,

    /// amount of delay [seconds] between sync attempts
    #[structopt(short = "r", long = "duration")]
    pub duration: Option<u64>,

    /// amount of delay [seconds] between sync attempts
    #[structopt(short = "n", long = "iterations")]
    pub sync_iteration: Option<u32>,

    #[structopt(short = "m", long = "measure")]
    pub measure: bool,
}

impl ClientLoadCommand {
    pub fn exec(&self) -> Result<(), ClientLoadCommandError> {
        let scenario_type = if let Some(duration) = self.duration {
            Some(ScenarioType::Duration(duration))
        } else {
            self.sync_iteration.map(ScenarioType::Iteration)
        };

        if scenario_type.is_none() {
            return Err(ClientLoadCommandError::NoScenarioDefined);
        }

        let config = self.build_config();

        Ok(PassiveBootstrapLoad::new(config).exec(scenario_type.unwrap())?)
    }

    fn get_block0_hash(&self) -> Hash {
        JormungandrClient::from_address(&self.address)
            .unwrap()
            .get_genesis_block_hash()
    }

    fn build_config(&self) -> ClientLoadConfig {
        let block0_hash = self.get_block0_hash();
        ClientLoadConfig::new(
            block0_hash,
            self.measure,
            self.count,
            self.address.clone(),
            self.pace,
            self.initial_storage.clone(),
        )
    }
}
