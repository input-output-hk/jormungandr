#![cfg(feature = "integration-test")]

mod common;
use common::configuration::jormungandr_config::JormungandrConfig;
use common::startup;

#[test]
pub fn test_jormungandr_node_starts_successfully() {
    let mut config = JormungandrConfig::new();
    let _jormungandr = startup::start_jormungandr_node(&mut config);
}
