mod scenario;

use crate::common::jormungandr::{JormungandrError, StartupError};
use chain_impl_mockchain::key::Hash;
use jormungandr_lib::interfaces::TrustedPeer;
use jormungandr_testing_utils::testing::node::RestError;
pub use scenario::{DurationBasedClientLoad, IterationBasedClientLoad};
use std::path::PathBuf;
use std::result::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientLoadError {
    #[error("cannot spawn not with version '{0}', looks like it's incorrect one")]
    VersionNotFound(String),
    #[error("cannot find node with alias(0). Please run 'describe' command ")]
    NodeAliasNotFound(String),
    #[error("cannot query rest")]
    RestError(#[from] RestError),
    #[error("cannot bootstrap node")]
    StartupError(#[from] StartupError),
    #[error("jormungandr error")]
    JormungandrError(#[from] JormungandrError),
    #[error("node client error")]
    InternalClientError,
    #[error("pace is too low ({0})")]
    PaceTooLow(u64),
}

pub struct PassiveBootstrapLoad {
    config: ClientLoadConfig,
}

impl PassiveBootstrapLoad {
    pub fn new(config: ClientLoadConfig) -> Self {
        Self { config }
    }

    pub fn exec(self, scenario: ScenarioType) -> Result<(), ClientLoadError> {
        if self.config.pace() < 2 {
            return Err(ClientLoadError::PaceTooLow(2));
        }

        match scenario {
            ScenarioType::Duration(duration) => {
                let duration_client_load = DurationBasedClientLoad::new(self.config, duration);
                duration_client_load.run()
            }
            ScenarioType::Iteration(iteration) => {
                IterationBasedClientLoad::new(self.config, iteration).run()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ScenarioType {
    Duration(u64),
    Iteration(u32),
}

#[derive(Debug, Clone)]
pub struct ClientLoadConfig {
    block0_hash: Hash,
    measure: bool,
    count: u32,
    address: String,
    ip: String,
    pace: u64,
    initial_storage: Option<PathBuf>,
}

impl ClientLoadConfig {
    pub fn new(
        block0_hash: Hash,
        measure: bool,
        count: u32,
        address: String,
        ip: String,
        pace: u64,
        initial_storage: Option<PathBuf>,
    ) -> Self {
        Self {
            block0_hash,
            measure,
            count,
            address,
            ip,
            pace,
            initial_storage,
        }
    }

    pub fn trusted_peer(&self) -> TrustedPeer {
        TrustedPeer {
            address: self.address.parse().unwrap(),
            id: None,
        }
    }

    pub fn pace(&self) -> u64 {
        self.pace
    }
}
