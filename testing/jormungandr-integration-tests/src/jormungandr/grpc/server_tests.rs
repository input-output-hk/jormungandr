use crate::common::{configuration, jormungandr::starter::Starter};

use jormungandr_testing_utils::testing::node::{
    grpc::server::{MethodType, MockBuilder, MockExitCode, ProtocolVersion},
    Level,
};

use super::setup::Fixture;
use chain_core::property::FromStr;
use chain_impl_mockchain::key::Hash;
use std::time::Duration;

// L1005 Handshake version discrepancy
#[tokio::test]
#[ignore]
pub async fn wrong_protocol() {
    let fixture = Fixture::default();

    let mock_port = configuration::get_available_port();
    let config = fixture.build_configuration(mock_port);

    let mock_controller = MockBuilder::new()
        .with_port(mock_port)
        .with_protocol_version(ProtocolVersion::Bft)
        .build();

    tokio::time::sleep(Duration::from_millis(1000)).await;

    let server = Starter::new().config(config.clone()).start_async().unwrap();

    let mock_result = mock_controller.finish_and_verify_that(|mock_verifier| {
        mock_verifier.method_executed_at_least_once(MethodType::Handshake)
    });
    server.shutdown();
    assert_eq!(
        mock_result,
        MockExitCode::Success,
        "Handshake with mock never happened"
    );
}

// L1004 Handshake hash discrepancy
#[tokio::test]
#[ignore]
pub async fn wrong_genesis_hash() {
    let fixture = Fixture::default();

    let mock_port = configuration::get_available_port();
    let config = fixture.build_configuration(mock_port);

    let mock_controller = MockBuilder::new()
        .with_port(mock_port)
        .with_protocol_version(ProtocolVersion::GenesisPraos)
        .build();

    tokio::time::sleep(Duration::from_millis(1000)).await;

    let server = Starter::new().config(config.clone()).start_async().unwrap();

    let mock_address = mock_controller.address();
    let mock_result = mock_controller.finish_and_verify_that(|mock_verifier| {
        mock_verifier.method_executed_at_least_once(MethodType::Handshake)
    });
    server.shutdown();
    assert_eq!(
        mock_result,
        MockExitCode::Success,
        "Handshake with mock never happened"
    );

    server.shutdown();
    assert!(
        server.logger.get_log_entries().any(|x| {
            x.fields.msg == "connection to peer failed"
                && x.error_contains("Block0Mismatch")
                && x.fields.peer_addr == Some(mock_address.clone())
                && x.level == Level::INFO
        }),
        "Log content: {}",
        server.logger.get_log_content()
    );
}

// L1002 Handshake compatible
#[tokio::test]
#[ignore]
pub async fn handshake_ok() {
    let fixture = Fixture::default();

    let mock_port = configuration::get_available_port();
    let config = fixture.build_configuration(mock_port);

    let mock_controller = MockBuilder::new()
        .with_port(mock_port)
        .with_genesis_hash(Hash::from_str(&config.genesis_block_hash()).unwrap())
        .with_protocol_version(ProtocolVersion::Bft)
        .build();

    tokio::time::sleep(Duration::from_millis(1000)).await;

    let server = Starter::new().config(config.clone()).start_async().unwrap();
    let mock_address = mock_controller.address();
    let mock_result = mock_controller.finish_and_verify_that(|mock_verifier| {
        mock_verifier.method_executed_at_least_once(MethodType::Handshake)
    });
    server.shutdown();

    assert_eq!(
        mock_result,
        MockExitCode::Success,
        "Handshake with mock never happened"
    );

    assert!(!server
        .logger
        .get_log_entries()
        .any(|x| { x.fields.peer_addr == Some(mock_address.clone()) && x.level == Level::WARN }));
}
