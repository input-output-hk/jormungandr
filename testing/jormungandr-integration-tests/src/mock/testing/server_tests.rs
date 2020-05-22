use crate::{
    common::{
        configuration, file_utils, jormungandr::logger::Level, jormungandr::starter::Starter,
    },
    mock::{
        client::JormungandrClient,
        server::{
            self, JormungandrServerImpl, MethodType, MockLogger, NodeServer, ProtocolVersion,
        },
        testing::setup::{
            bootstrap_node_with_peer, build_configuration, start_mock, MockController, MockExitCode,
        },
    },
};
use chain_core::property::FromStr;
use chain_impl_mockchain::key::Hash;
use futures_util::future::FutureExt;
use std::{
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
use tonic::transport::Server;

pub fn fake_hash() -> Hash {
    Hash::from_str("efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944").unwrap()
}

pub fn peer_addr(port: u16) -> Option<String> {
    Some(format!("127.0.0.1:{}", port))
}

// L1005 Handshake version discrepancy
#[tokio::test]
#[ignore]
pub async fn wrong_protocol() {
    let mock_port = configuration::get_available_port();
    let config = build_configuration(mock_port);

    let mock_controller = start_mock(
        mock_port,
        Hash::from_str(&config.genesis_block_hash()).unwrap(),
        fake_hash(),
        ProtocolVersion::Bft,
    );
    tokio::time::delay_for(Duration::from_millis(1000)).await;

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
    let mock_port = configuration::get_available_port();
    let config = build_configuration(mock_port);

    let mock_controller = start_mock(
        mock_port,
        fake_hash(),
        fake_hash(),
        ProtocolVersion::GenesisPraos,
    );
    tokio::time::delay_for(Duration::from_millis(1000)).await;

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

    server.shutdown();
    assert!(
        server.logger.get_log_entries().any(|x| {
            x.msg == "connection to peer failed"
                && x.error_contains("Block0Mismatch")
                && x.peer_addr == peer_addr(mock_port)
                && x.level == Level::INFO
        }),
        format!("Log content: {}", server.logger.get_log_content())
    );
}

// L1002 Handshake compatible
#[tokio::test]
#[ignore]
pub async fn handshake_ok() {
    let mock_port = configuration::get_available_port();
    let config = build_configuration(mock_port);

    let mock_controller = start_mock(
        mock_port,
        Hash::from_str(&config.genesis_block_hash()).unwrap(),
        fake_hash(),
        ProtocolVersion::GenesisPraos,
    );
    tokio::time::delay_for(Duration::from_millis(1000)).await;

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

    assert!(!server
        .logger
        .get_log_entries()
        .any(|x| { x.peer_addr == peer_addr(mock_port) && x.level == Level::WARN }));
}
