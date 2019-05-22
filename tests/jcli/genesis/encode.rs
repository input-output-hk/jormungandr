#![cfg(feature = "integration-test")]

use common::configuration::genesis_model::GenesisYaml;
use common::startup;

#[test]
pub fn test_genesis_block_is_built_from_corect_yaml() {
    startup::build_genesis_block(&GenesisYaml::new());
}
