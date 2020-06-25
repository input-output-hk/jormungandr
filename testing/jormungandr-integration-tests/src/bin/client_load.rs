use chain_impl_mockchain::key::Hash;
use jormungandr_integration_tests::{
    common::load::{ClientLoadConfig, ClientLoadError, PassiveBootstrapLoad, ScenarioType},
    mock::client::JormungandrClient,
};
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

pub fn main() -> Result<(), ClientLoadCommandError> {
    ClientLoadCommand::from_args().exec()
}

#[derive(Error, Debug)]
pub enum ClientLoadCommandError {
    #[error("No scenario defined for run. Available: [duration,iteration]")]
    NoScenarioDefined,
    #[error("Client Error")]
    ClientError(#[from] ClientLoadError),
}

#[derive(StructOpt, Debug)]
pub struct ClientLoadCommand {
    /// Prints nodes related data, like stats,fragments etc.
    #[structopt(short = "c", long = "count", default_value = "3")]
    pub count: u32,
    /// address in format:
    /// /ip4/54.193.75.55/tcp/3000
    #[structopt(short = "a", long = "address")]
    pub address: String,

    #[structopt(short = "i", long = "ip", default_value = "127.0.0.1")]
    pub ip: String,

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
        } else if let Some(iteration) = self.sync_iteration {
            Some(ScenarioType::Iteration(iteration))
        } else {
            None
        };

        if let None = scenario_type {
            return Err(ClientLoadCommandError::NoScenarioDefined);
        }

        let config = self.build_config();

        Ok(PassiveBootstrapLoad::new(config).exec(scenario_type.unwrap())?)
    }

    fn get_block0_hash(&self) -> Hash {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                let grpc_client = JormungandrClient::from_address(&self.address).unwrap();
                return grpc_client.get_genesis_block_hash().await;
            })
            .into()
    }

    fn build_config(&self) -> ClientLoadConfig {
        let block0_hash = self.get_block0_hash();
        ClientLoadConfig::new(
            block0_hash,
            self.measure,
            self.count,
            self.address.clone(),
            self.ip.clone(),
            self.pace,
            self.initial_storage.clone(),
        )
    }
}
