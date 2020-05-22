use crate::common::{
    configuration::jormungandr_config::JormungandrConfig,
    file_utils,
    jormungandr::{ConfigurationBuilder, JormungandrProcess, Starter},
};
use crate::mock::{
    client::JormungandrClient,
    server::{JormungandrServerImpl, MethodType, MockLogger, NodeServer, ProtocolVersion},
};
use chain_impl_mockchain::chaintypes::ConsensusVersion;
use chain_impl_mockchain::key::Hash;
use futures_util::future::FutureExt;
use jormungandr_lib::interfaces::TrustedPeer;
use std::path::PathBuf;
use std::{
    thread,
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tonic::transport::Server;

const LOCALHOST: &str = "127.0.0.1";

pub struct Config {
    host: String,
    port: u16,
}

impl Config {
    pub fn attach_to_local_node(port: u16) -> Self {
        Self {
            host: String::from(LOCALHOST),
            port: port,
        }
    }

    pub fn client(&self) -> JormungandrClient {
        JormungandrClient::new(&self.host, self.port)
    }
}

pub fn bootstrap_node() -> (JormungandrProcess, JormungandrConfig) {
    let config = ConfigurationBuilder::new().with_slot_duration(4).build();
    let server = Starter::new().config(config.clone()).start_async().unwrap();
    thread::sleep(Duration::from_secs(4));
    (server, config)
}

pub fn build_configuration(mock_port: u16) -> JormungandrConfig {
    let trusted_peer = TrustedPeer {
        address: format!("/ip4/{}/tcp/{}", LOCALHOST, mock_port)
            .parse()
            .unwrap(),
    };

    ConfigurationBuilder::new()
        .with_slot_duration(4)
        .with_block0_consensus(ConsensusVersion::GenesisPraos)
        .with_trusted_peers(vec![trusted_peer])
        .build()
}

pub fn bootstrap_node_with_peer(mock_port: u16) -> (JormungandrProcess, JormungandrConfig) {
    let config = build_configuration(mock_port);
    let server = Starter::new().config(config.clone()).start_async().unwrap();
    thread::sleep(Duration::from_secs(4));
    (server, config)
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
    let log_file = file_utils::get_path_in_temp("mock.log");
    println!("mock will put logs into path: {:?}", log_file);

    let logger = MockLogger::new(log_file.clone());
    let (shutdown_signal, rx) = oneshot::channel::<()>();

    let join_handle = tokio::spawn(async move {
        let addr = format!("127.0.0.1:{}", mock_port);
        let mock = JormungandrServerImpl::new(
            mock_port,
            genesis_hash,
            tip_hash,
            protocol_version,
            log_file,
        );

        Server::builder()
            .add_service(NodeServer::new(mock))
            .serve_with_shutdown(addr.parse().unwrap(), rx.map(drop))
            .await
            .unwrap();
    });
    MockController::new(logger, shutdown_signal)
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
}

impl MockController {
    pub fn new(logger: MockLogger, stop_signal: tokio::sync::oneshot::Sender<()>) -> Self {
        Self {
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
