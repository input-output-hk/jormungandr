#![cfg(feature = "integration-test")]

mod common;
use common::startup;

#[test]
pub fn test_jormungandr_node_starts_successfully() {
    let mut config = startup::build_configuration();
    let _jormungandr = startup::start_jormungandr_node(&mut config);
}
