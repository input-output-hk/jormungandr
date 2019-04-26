#![cfg(feature = "integration-test")]

mod common;

use common::configuration::genesis_model::GenesisYaml;
use common::configuration::node_config_model::NodeConfig;
use common::startup;

#[test]
pub fn test_jormungandr_node_starts_successfully() {
    let _guard =
        startup::start_jormungandr_node_with_genesis_conf(&GenesisYaml::new(), &NodeConfig::new());
}
