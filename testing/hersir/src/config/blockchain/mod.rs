mod configuration;
pub use chain_impl_mockchain::chaintypes::ConsensusVersion;
pub use configuration::{BlockchainBuilder, BlockchainConfiguration};
use jormungandr_lib::crypto::hash::Hash;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum BlockchainConfigurationOrHash {
    Block0(BlockchainConfiguration),
    Block0Hash(Hash),
}

impl Default for BlockchainConfigurationOrHash {
    fn default() -> Self {
        Self::Block0(BlockchainConfiguration::default())
    }
}
