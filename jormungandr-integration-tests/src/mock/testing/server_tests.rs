use crate::{
    common::{
        configuration, file_utils, jormungandr::logger::Level,
        jormungandr::starter::{Starter,StartupVerificationMode},
    },
    mock::{
        server::{self, MethodType, MockLogger, ProtocolVersion},
        testing::{setup::bootstrap_node_with_peer, setup::build_configuration},
    },
};
use chain_core::property::FromStr;
use chain_impl_mockchain::key::Hash;
use std::{
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq)]
pub enum MockExitCode {
    Timeout,
    Success,
}

pub fn start_mock<F: 'static>(
    mock_port: u16,
    genesis_hash: Hash,
    tip_hash: Hash,
    protocol_version: ProtocolVersion,
    stop_func: F,
) -> JoinHandle<MockExitCode>
where
    F: Fn(&MockLogger) -> bool,
    F: std::marker::Send,
{
    let log_file = file_utils::get_path_in_temp("mock.log");
    let logger = MockLogger::new(log_file.clone());

    thread::spawn(move || {
        let _server = server::start(
            mock_port,
            genesis_hash,
            tip_hash,
            protocol_version,
            log_file.clone(),
        );

        let start = Instant::now();
        let timeout = Duration::from_secs(120);

        loop {
            thread::sleep(Duration::from_secs(1));
            if start.elapsed() > timeout {
                return MockExitCode::Timeout;
            }
            if stop_func(&logger) {
                return MockExitCode::Success;
            }
        }
    })
}

const FAKE_HASH: &str = "efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944";

pub fn fake_hash() -> Hash {
    Hash::from_str(FAKE_HASH).unwrap()
}

pub fn peer_addr(port: u16) -> Option<String> {
    Some(format!("127.0.0.1:{}", port))
}

// L1005 Handshake version discrepancy
#[test]
pub fn wrong_protocol() {
    let mock_port = configuration::get_available_port();
    let config = build_configuration(mock_port);

    let mock_thread = start_mock(
        mock_port,
        Hash::from_str(&config.genesis_block_hash).unwrap(),
        fake_hash(),
        ProtocolVersion::Bft,
        |logger: &MockLogger| logger.executed_at_least_once(MethodType::Handshake),
    );

    let server = Starter::new()
                    .passive()
                    .config(config)
                    .verify_by(StartupVerificationMode::Log)
                    .start()
                    .unwrap();

    assert_eq!(
        mock_thread.join().expect("mock thread error"),
        MockExitCode::Success,
        "Mock server timeout while waiting to stop event be triggered"
    );

    server.shutdown();
    assert!(
        server.logger.get_log_entries().any(|x| {
            x.msg == "failed to connect to peer"
                && x.reason_contains("protocol handshake failed: unsupported protocol version 0")
                && x.peer_addr == peer_addr(mock_port)
                && x.level == Level::INFO
        }),
        format!("Log content: {}", server.logger.get_log_content())
    );
}

// L1004 Handshake hash discrepancy
#[test]
pub fn wrong_genesis_hash() {
    let mock_port = configuration::get_available_port();
    let mock_thread = start_mock(
        mock_port,
        fake_hash(),
        fake_hash(),
        ProtocolVersion::GenesisPraos,
        |logger: &MockLogger| logger.executed_at_least_once(MethodType::Handshake),
    );

    let (server, _) = bootstrap_node_with_peer(mock_port);
    assert_eq!(
        mock_thread.join().expect("mock thread error"),
        MockExitCode::Success
    );

    server.shutdown();
    assert!(
        server.logger.get_log_entries().any(|x| {
            x.msg == "failed to connect to peer"
                && x.reason_contains("genesis block hash")
                && x.peer_addr == peer_addr(mock_port)
                && x.level == Level::INFO
        }),
        format!("Log content: {}", server.logger.get_log_content())
    );
}

// L1002 Handshake compatible
#[test]
pub fn handshake_ok() {
    let mock_port = configuration::get_available_port();
    let config = build_configuration(mock_port);

    let mock_thread = start_mock(
        mock_port,
        Hash::from_str(&config.genesis_block_hash).unwrap(),
        fake_hash(),
        ProtocolVersion::GenesisPraos,
        |logger: &MockLogger| logger.executed_at_least_once(MethodType::Handshake),
    );

    let server = Starter::new().config(config.clone()).start().unwrap();
    assert_eq!(
        mock_thread.join().expect("mock thread error"),
        MockExitCode::Success
    );

    server.shutdown();
    server.logger.print_error_or_warn_lines();
    assert!(!server
        .logger
        .get_log_entries()
        .any(|x| { x.peer_addr == peer_addr(mock_port) && x.level == Level::WARN }));
}

//L1008 Tip request hash discrepancy
#[ignore] // ignored until issues with missing header will be resolved
#[test]
pub fn tip_request_malformed_discrepancy() {
    let mock_port = configuration::get_available_port();
    let config = build_configuration(mock_port);

    let mock_thread = start_mock(
        mock_port,
        Hash::from_str(&config.genesis_block_hash).unwrap(),
        fake_hash(),
        ProtocolVersion::GenesisPraos,
        |logger: &MockLogger| logger.executed_at_least_once(MethodType::Tip),
    );

    let server = Starter::new().config(config.clone()).start().unwrap();
    assert_eq!(
        mock_thread.join().expect("mock thread error"),
        MockExitCode::Success
    );

    server.shutdown();
    server.logger.print_error_or_warn_lines();
    assert!(!server
        .logger
        .get_log_entries()
        .any(|x| { x.peer_addr == peer_addr(mock_port) && x.level == Level::WARN }));
}
