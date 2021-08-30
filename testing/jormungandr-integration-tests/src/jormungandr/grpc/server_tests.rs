use super::setup;
use jormungandr_testing_utils::testing::node::{
    grpc::server::{MethodType, MockBuilder, MockExitCode, ProtocolVersion},
    LogLevel,
};

// L1005 Handshake version discrepancy
#[test]
pub fn wrong_protocol() {
    let setup = setup::server::default();

    let block0 = setup.server.block0_configuration().to_block();
    let genesis_hash = setup.server.genesis_block_hash().into_hash();

    let mock_controller = MockBuilder::new()
        .with_port(setup.mock_port)
        .with_genesis_hash(genesis_hash)
        .with_tip(block0.header().clone())
        .with_protocol_version(ProtocolVersion::Bft)
        .build();

    setup.wait_server_online();

    let mock_result = mock_controller.finish_and_verify_that(|mock_verifier| {
        mock_verifier.method_executed_at_least_once(MethodType::Handshake)
    });
    setup.server.shutdown();
    assert_eq!(
        mock_result,
        MockExitCode::Success,
        "Handshake with mock never happened"
    );

    assert!(setup.server.logger.get_lines().into_iter().any(|x| {
        x.message() == "protocol handshake with peer failed"
            && x.reason_contains("unsupported protocol version")
    }));
}

// L1004 Handshake hash discrepancy
#[test]
pub fn wrong_genesis_hash() {
    let setup = setup::server::default();

    let block0 = setup.server.block0_configuration().to_block();

    let mock_controller = MockBuilder::new()
        .with_port(setup.mock_port)
        // little hack, we need tip header to be the same of the server so that we can complete bootstrap
        .with_tip(block0.header().clone())
        .with_protocol_version(ProtocolVersion::GenesisPraos)
        .build();

    setup.wait_server_online();

    let mock_result = mock_controller.finish_and_verify_that(|mock_verifier| {
        mock_verifier.method_executed_at_least_once(MethodType::Handshake)
    });
    setup.server.shutdown();
    assert_eq!(
        mock_result,
        MockExitCode::Success,
        "Handshake with mock never happened"
    );

    assert!(
        setup.server.logger.get_lines().into_iter().any(|x| {
            x.message() == "connection to peer failed"
                && x.error_contains("Block0Mismatch")
                && x.level == LogLevel::INFO
        }),
        "Log content: {}",
        setup.server.logger.get_log_content()
    );
}

// L1002 Handshake compatible
#[test]
pub fn handshake_ok() {
    let setup = setup::server::default();

    let block0 = setup.server.block0_configuration().to_block();
    let genesis_hash = setup.server.genesis_block_hash().into_hash();

    let mock_controller = MockBuilder::new()
        .with_port(setup.mock_port)
        .with_genesis_hash(genesis_hash)
        .with_tip(block0.header().clone())
        .with_protocol_version(ProtocolVersion::GenesisPraos)
        .build();

    setup.wait_server_online();

    let mock_result = mock_controller.finish_and_verify_that(|mock_verifier| {
        mock_verifier.method_executed_at_least_once(MethodType::Handshake)
    });

    setup.server.shutdown();

    assert_eq!(
        mock_result,
        MockExitCode::Success,
        "Handshake with mock never happened"
    );

    assert!(!setup.server.logger.get_lines().into_iter().any(|x| {
        x.message() == "protocol handshake with peer failed"
            && x.reason_contains("unsupported protocol version")
    }));
}
