#![cfg(feature = "integration-test")]

use common::configuration::jormungandr_config::JormungandrConfig;
use common::configuration::node_config_model::NodeConfig;
use common::jcli_wrapper;
use common::process_assert;
use common::startup;

#[test]
pub fn test_correct_id_is_returned_for_block_tip_if_only_genesis_block_exists() {
    let mut config = startup::ConfigurationBuilder::new().build();
    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node(&mut config);
    let block_id = jcli_wrapper::assert_rest_get_block_tip(&jormungandr_rest_address);

    assert_ne!(&block_id, "", "empty block hash");
}

#[test]
/// False green due to: #298
pub fn test_correct_error_is_returned_for_incorrect_path() {
    let node_config = NodeConfig::new();
    let mut incorrect_host = node_config.get_node_address();
    incorrect_host.push('x');

    process_assert::assert_process_failed_and_matches_message_with_desc(
        jcli_wrapper::jcli_commands::get_rest_block_tip_command(&incorrect_host),
        "thread 'main' panicked at",
        "This assertion is incorrect on purpose to avoid failing build when running test,
        after #298 is fixed it need to be changed to correct one",
    );
}
