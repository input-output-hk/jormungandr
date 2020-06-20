use crate::common::{
    configuration::jormungandr_config::JormungandrParams,
    jormungandr::{ConfigurationBuilder, JormungandrProcess, Starter},
};
use crate::mock::{
    client::JormungandrClient,
    server::{JormungandrServerImpl, MethodType, MockLogger, NodeServer, ProtocolVersion},
};
use chain_impl_mockchain::chaintypes::ConsensusVersion;
use chain_impl_mockchain::key::Hash;
use jormungandr_lib::interfaces::TrustedPeer;

use assert_fs::prelude::*;
use assert_fs::TempDir;
use futures::future::FutureExt;
use tokio::sync::oneshot;
use tonic::transport::Server;

use std::thread;
use std::time::{Duration, Instant};

const LOCALHOST: &str = "127.0.0.1";

pub struct Config {
    host: String,
    port: u16,
}

impl Config {
    pub fn attach_to_local_node(port: u16) -> Self {
        Self {
            host: String::from(LOCALHOST),
            port,
        }
    }

    pub fn client(&self) -> JormungandrClient {
        JormungandrClient::new(&self.host, self.port)
    }
}

pub struct Fixture {
    temp_dir: TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        Fixture { temp_dir }
    }

    pub fn bootstrap_node(&self) -> (JormungandrProcess, JormungandrParams) {
        let config = ConfigurationBuilder::new()
            .with_slot_duration(4)
            .build(&self.temp_dir);
        let server = Starter::new().config(config.clone()).start_async().unwrap();
        thread::sleep(Duration::from_secs(4));
        (server, config)
    }

    pub fn build_configuration(&self, mock_port: u16) -> JormungandrParams {
        let trusted_peer = TrustedPeer {
            address: format!("/ip4/{}/tcp/{}", LOCALHOST, mock_port)
                .parse()
                .unwrap(),
            id: None,
        };

        ConfigurationBuilder::new()
            .with_slot_duration(4)
            .with_block0_consensus(ConsensusVersion::GenesisPraos)
            .with_trusted_peers(vec![trusted_peer])
            .build(&self.temp_dir)
    }

    pub fn bootstrap_node_with_peer(
        &self,
        mock_port: u16,
    ) -> (JormungandrProcess, JormungandrParams) {
        let config = self.build_configuration(mock_port);
        let server = Starter::new().config(config.clone()).start_async().unwrap();
        thread::sleep(Duration::from_secs(4));
        (server, config)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MockExitCode {
    Timeout,
    Success,
}

pub fn start_mock(
    mock_port: u16,
    genesis_hash: Hash,
    tip_hash: Hash,
    protocol_version: ProtocolVersion,
) -> MockController {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.child("mock.log");
    println!(
        "mock will put logs into {}",
        log_file.path().to_string_lossy()
    );

    let logger = MockLogger::new(log_file.path());
    let (shutdown_signal, rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        let addr = format!("127.0.0.1:{}", mock_port);
        let mock = JormungandrServerImpl::new(
            mock_port,
            genesis_hash,
            tip_hash,
            protocol_version,
            log_file.path(),
        );

        Server::builder()
            .add_service(NodeServer::new(mock))
            .serve_with_shutdown(addr.parse().unwrap(), rx.map(drop))
            .await
            .unwrap();
    });
    MockController::new(temp_dir, logger, shutdown_signal)
}

pub struct MockVerifier {
    logger: MockLogger,
}

impl MockVerifier {
    pub fn new(logger: MockLogger) -> Self {
        Self { logger: logger }
    }

    pub fn method_executed_at_least_once(&self, method: MethodType) -> bool {
        self.logger.executed_at_least_once(method)
    }
}

pub struct MockController {
    verifier: MockVerifier,
    stop_signal: tokio::sync::oneshot::Sender<()>,
    // only need to keep this for the lifetime of the fixture
    #[allow(dead_code)]
    temp_dir: TempDir,
}

impl MockController {
    fn new(
        temp_dir: TempDir,
        logger: MockLogger,
        stop_signal: tokio::sync::oneshot::Sender<()>,
    ) -> Self {
        Self {
            temp_dir,
            verifier: MockVerifier::new(logger),
            stop_signal: stop_signal,
        }
    }

    pub fn finish_and_verify_that<F: 'static + std::marker::Send>(
        self,
        verify_func: F,
    ) -> MockExitCode
    where
        F: Fn(&MockVerifier) -> bool,
    {
        let start = Instant::now();
        let timeout = Duration::from_secs(120);

        loop {
            thread::sleep(Duration::from_secs(1));
            if start.elapsed() > timeout {
                self.stop();
                return MockExitCode::Timeout;
            }
            if verify_func(&self.verifier) {
                self.stop();
                return MockExitCode::Success;
            }
        }
    }

    pub fn stop(self) {
        self.stop_signal.send(()).unwrap();
    }
}
