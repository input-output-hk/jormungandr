mod common;

use common::configuration::genesis_model::GenesisYaml;
use common::startup;

#[test]
#[cfg(feature = "integration-test")]
pub fn test_jormungandr_node_starts_successfully() {
    startup::start_jormungandr_node_with_genesis_conf(GenesisYaml::new());
}
