use crate::mjolnir_lib::{
    bootstrap::scenario::{DurationBasedClientLoad, IterationBasedClientLoad},
    MjolnirError,
};
use chain_impl_mockchain::key::Hash;
use jormungandr_lib::interfaces::TrustedPeer;
use std::{path::PathBuf, result::Result};
pub struct PassiveBootstrapLoad {
    config: ClientLoadConfig,
}

impl PassiveBootstrapLoad {
    pub fn new(config: ClientLoadConfig) -> Self {
        Self { config }
    }

    pub fn exec(self, scenario: ScenarioType) -> Result<(), MjolnirError> {
        if self.config.pace() < 2 {
            return Err(MjolnirError::PaceTooLow(2));
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
    pace: u64,
    initial_storage: Option<PathBuf>,
}

impl ClientLoadConfig {
    pub fn new(
        block0_hash: Hash,
        measure: bool,
        count: u32,
        address: String,
        pace: u64,
        initial_storage: Option<PathBuf>,
    ) -> Self {
        Self {
            block0_hash,
            measure,
            count,
            address,
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

    pub fn block0_hash(&self) -> &Hash {
        &self.block0_hash
    }

    pub fn measure(&self) -> bool {
        self.measure
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn initial_storage(&self) -> &Option<PathBuf> {
        &self.initial_storage
    }
}
