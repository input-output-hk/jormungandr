use super::setup;
use assert_fs::TempDir;
use jormungandr_automation::jormungandr::{
    grpc::server::{MethodType, MockBuilder, MockExitCode, ProtocolVersion},
    LogLevel,
};

// L1005 Handshake version discrepancy
#[test]
pub fn wrong_protocol() {
    let temp_dir = TempDir::new().unwrap().into_persistent();
    let setup = setup::server::default();

    let mock_controller = MockBuilder::default()
        .with_port(setup.mock_port)
        .with_genesis_block(setup.test_context.block0_config.to_block())
        .with_protocol_version(ProtocolVersion::Bft)
        .build();

    let jormungandr = setup.test_context.start_node(temp_dir).unwrap();

    //setup.wait_server_online();

    let mock_result = mock_controller.finish_and_verify_that(|mock_verifier| {
        mock_verifier.method_executed_at_least_once(MethodType::Handshake)
    });
    jormungandr.shutdown();
    assert_eq!(
        mock_result,
        MockExitCode::Success,
        "Handshake with mock never happened"
    );

    assert!(jormungandr.logger.get_lines().into_iter().any(|x| {
        x.message() == "protocol handshake with peer failed"
            && x.reason_contains("unsupported protocol version")
    }));
}

// L1004 Handshake hash discrepancy
#[test]
pub fn wrong_genesis_hash() {
    let setup = setup::server::default();
    let temp_dir = TempDir::new().unwrap().into_persistent();
    let block0 = setup.test_context.block0_config.to_block();

    let mut mock_controller = MockBuilder::default()
        .with_port(setup.mock_port)
        .with_protocol_version(ProtocolVersion::GenesisPraos)
        .with_invalid_block0_hash(true)
        .build();
    mock_controller.set_tip_block(&block0);
    let jormungandr = setup.test_context.start_node(temp_dir).unwrap();

    let mock_result = mock_controller.finish_and_verify_that(|mock_verifier| {
        mock_verifier.method_executed_at_least_once(MethodType::Handshake)
    });
    jormungandr.shutdown();
    assert_eq!(
        mock_result,
        MockExitCode::Success,
        "Handshake with mock never happened"
    );

    assert!(
        jormungandr.logger.get_lines().into_iter().any(|x| {
            x.message() == "connection to peer failed"
                && x.error_contains("Block0Mismatch")
                && x.level == LogLevel::INFO
        }),
        "Log content: {}",
        jormungandr.logger.get_log_content()
    );
}

// L1002 Handshake compatible
#[test]
pub fn handshake_ok() {
    let setup = setup::server::default();
    let temp_dir = TempDir::new().unwrap().into_persistent();
    let block0 = setup.test_context.block0_config.to_block();

    let mock_controller = MockBuilder::default()
        .with_port(setup.mock_port)
        .with_genesis_block(block0)
        .with_protocol_version(ProtocolVersion::GenesisPraos)
        .build();

    let jormungandr = setup.test_context.start_node(temp_dir).unwrap();

    let mock_result = mock_controller.finish_and_verify_that(|mock_verifier| {
        mock_verifier.method_executed_at_least_once(MethodType::Handshake)
    });

    jormungandr.shutdown();

    assert_eq!(
        mock_result,
        MockExitCode::Success,
        "Handshake with mock never happened"
    );

    assert!(!jormungandr.logger.get_lines().into_iter().any(|x| {
        x.message() == "protocol handshake with peer failed"
            && x.reason_contains("unsupported protocol version")
    }));
}
