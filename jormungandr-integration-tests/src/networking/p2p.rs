use crate::common::network::{builder, params, wallet};

use jormungandr_lib::interfaces::Explorer;

#[test]
pub fn node_whitelist_itself() {
    let mut network_controller = builder("node_whitelist_itself")
        .single_trust_direction("client", "server")
        .initials(vec![
            wallet("delegated1").with(1_000_000).delegated_to("client"),
            wallet("delegated2").with(1_000_000).delegated_to("server"),
        ])
        .custom_config(vec![params("client").explorer(Explorer { enabled: true })])
        .build()
        .unwrap();

    let server = network_controller.spawn_and_wait("server");
    let client = network_controller.spawn_and_wait("client");

    client.assert_no_errors_in_log();
}
