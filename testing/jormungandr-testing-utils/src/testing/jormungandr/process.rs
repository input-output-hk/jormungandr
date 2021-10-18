use super::{starter::StartupError, JormungandrError};
use crate::testing::jcli::{JCli, JCliCommand};
use crate::testing::{
    node::{
        uri_from_socket_addr, Explorer, JormungandrLogger, JormungandrRest,
        JormungandrStateVerifier, LogLevel,
    },
    utils, BlockDateGenerator, JormungandrParams, SyncNode, TestConfig,
};
use crate::testing::{
    FragmentChainSender, FragmentSender, FragmentSenderSetup, RemoteJormungandr,
    RemoteJormungandrBuilder,
};
use ::multiaddr::Multiaddr;
use assert_fs::TempDir;
use chain_impl_mockchain::{block::BlockDate, fee::LinearFee};
use chain_time::TimeEra;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Block0Configuration, TrustedPeer},
};
use jortestkit::prelude::ProcessOutput;
use std::net::SocketAddr;
use std::path::Path;
use std::process::Child;
use std::process::Stdio;
use std::str::FromStr;

use std::time::{Duration, Instant};

pub enum StartupVerificationMode {
    Log,
    Rest,
}

pub enum Status {
    Running,
    Starting,
    Stopped(JormungandrError),
}

pub struct JormungandrProcess {
    pub child: Child,
    pub logger: JormungandrLogger,
    temp_dir: Option<TempDir>,
    alias: String,
    p2p_public_address: Multiaddr,
    rest_socket_addr: SocketAddr,
    genesis_block_hash: Hash,
    block0_configuration: Block0Configuration,
    fees: LinearFee,
}

impl JormungandrProcess {
    pub(crate) fn from_config<Conf: TestConfig>(
        mut child: Child,
        params: &JormungandrParams<Conf>,
        temp_dir: Option<TempDir>,
        alias: String,
    ) -> Result<Self, StartupError> {
        let node_config = params.node_config();
        let stdout = child.stdout.take().unwrap();
        Ok(JormungandrProcess {
            child,
            temp_dir,
            alias,
            logger: JormungandrLogger::new(stdout),
            p2p_public_address: node_config.p2p_public_address(),
            rest_socket_addr: node_config.rest_socket_addr(),
            genesis_block_hash: Hash::from_str(params.genesis_block_hash())?,
            block0_configuration: params.block0_configuration().clone(),
            fees: params.fees(),
        })
    }

    pub fn fragment_sender<'a, S: SyncNode + Send>(
        &self,
        setup: FragmentSenderSetup<'a, S>,
    ) -> FragmentSender<'a, S> {
        FragmentSender::new(
            self.genesis_block_hash(),
            self.fees(),
            self.default_block_date_generator(),
            setup,
        )
    }

    pub fn default_block_date_generator(&self) -> BlockDateGenerator {
        BlockDateGenerator::Rolling {
            block0_time: self
                .block0_configuration
                .blockchain_configuration
                .block0_date
                .into(),
            slot_duration: {
                let slot_duration: u8 = self
                    .block0_configuration
                    .blockchain_configuration
                    .slot_duration
                    .into();
                slot_duration.into()
            },
            slots_per_epoch: self
                .block0_configuration
                .blockchain_configuration
                .slots_per_epoch
                .into(),
            shift: BlockDate {
                epoch: 1,
                slot_id: 0,
            },
            shift_back: false,
        }
    }

    pub fn fragment_chain_sender<'a, S: SyncNode + Send>(
        &self,
        setup: FragmentSenderSetup<'a, S>,
    ) -> FragmentChainSender<'a, S> {
        FragmentChainSender::new(
            self.genesis_block_hash(),
            self.fees(),
            self.default_block_date_generator(),
            setup,
            self.to_remote(),
        )
    }

    pub fn wait_for_bootstrap(
        &self,
        verification_mode: &StartupVerificationMode,
        timeout: Duration,
    ) -> Result<(), StartupError> {
        let start = Instant::now();
        loop {
            if start.elapsed() > timeout {
                return Err(StartupError::Timeout {
                    timeout: timeout.as_secs(),
                    log_content: self.logger.get_log_content(),
                });
            }
            match self.status(verification_mode) {
                Status::Running => {
                    println!("jormungandr is up");
                    return Ok(());
                }
                Status::Stopped(err) => {
                    println!("attempt stopped due to error signal recieved");
                    println!("Raw log:\n {}", self.logger.get_log_content());
                    return Err(StartupError::JormungandrError(err));
                }
                Status::Starting => {}
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    fn check_startup_errors_in_logs(&self) -> Result<(), JormungandrError> {
        let port_occupied_msgs = ["error 87", "error 98", "panicked at 'Box<Any>'"];
        if self.logger.contains_any_of(&port_occupied_msgs) {
            return Err(JormungandrError::PortAlreadyInUse);
        }

        self.check_no_errors_in_log()
    }

    fn status(&self, strategy: &StartupVerificationMode) -> Status {
        match strategy {
            StartupVerificationMode::Log => {
                let bootstrap_completed_msgs = [
                    "listening and accepting gRPC connections",
                    "genesis block fetched",
                ];
                if let Err(err) = self.check_startup_errors_in_logs() {
                    Status::Stopped(err)
                } else if self.logger.contains_any_of(&bootstrap_completed_msgs) {
                    Status::Running
                } else {
                    Status::Starting
                }
            }
            StartupVerificationMode::Rest => {
                let jcli: JCli = Default::default();

                let output = JCliCommand::new(std::process::Command::new(jcli.path()))
                    .rest()
                    .v0()
                    .node()
                    .stats(&self.rest_uri())
                    .build()
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap()
                    .wait_with_output()
                    .expect("failed to execute get_rest_stats command");

                let output = output.try_as_single_node_yaml();
                match output
                    .ok()
                    .as_ref()
                    .and_then(|x| x.get("state"))
                    .map(|x| x.as_str())
                {
                    Some("Running") => Status::Running,
                    _ => {
                        if let Err(err) = self.check_startup_errors_in_logs() {
                            Status::Stopped(err)
                        } else {
                            Status::Starting
                        }
                    }
                }
            }
        }
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn rest(&self) -> JormungandrRest {
        JormungandrRest::new(self.rest_uri())
    }

    pub fn rest_debug(&self) -> JormungandrRest {
        let mut rest = JormungandrRest::new(self.rest_uri());
        rest.enable_logger();
        rest
    }

    pub fn secure_rest<P: AsRef<Path>>(&self, cert: P) -> JormungandrRest {
        JormungandrRest::new_with_cert(self.rest_uri(), cert)
    }

    pub fn shutdown(&self) {
        let jcli: JCli = Default::default();
        jcli.rest().v0().shutdown(self.rest_uri());
    }

    pub fn address(&self) -> SocketAddr {
        jormungandr_lib::multiaddr::to_tcp_socket_addr(&self.p2p_public_address).unwrap()
    }

    pub fn correct_state_verifier(&self) -> JormungandrStateVerifier {
        JormungandrStateVerifier::new(self.rest())
    }

    pub fn log_stats(&self) {
        println!("{:?}", self.rest().stats());
    }

    pub fn assert_no_errors_in_log_with_message(&self, message: &str) {
        self.logger.assert_no_errors(message);
    }

    pub fn assert_no_errors_in_log(&self) {
        self.logger.assert_no_errors("");
    }

    pub fn check_no_errors_in_log(&self) -> Result<(), JormungandrError> {
        let error_lines = self
            .logger
            .get_lines_with_level(LogLevel::ERROR)
            .collect::<Vec<_>>();

        if !error_lines.is_empty() {
            return Err(JormungandrError::ErrorInLogs {
                logs: self.logger.get_log_content(),
                error_lines: format!("{:?}", error_lines),
            });
        }
        Ok(())
    }

    pub fn rest_uri(&self) -> String {
        uri_from_socket_addr(self.rest_socket_addr)
    }

    pub fn fees(&self) -> LinearFee {
        self.fees
    }

    pub fn genesis_block_hash(&self) -> Hash {
        self.genesis_block_hash
    }

    pub fn block0_configuration(&self) -> &Block0Configuration {
        &self.block0_configuration
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn explorer(&self) -> Explorer {
        Explorer::new(self.rest_socket_addr.to_string())
    }

    pub fn to_trusted_peer(&self) -> TrustedPeer {
        TrustedPeer {
            address: self.p2p_public_address.clone(),
            id: None,
        }
    }

    pub fn time_era(&self) -> TimeEra {
        let block_date = self.explorer().current_time();

        TimeEra::new(
            (block_date.slot() as u64).into(),
            chain_time::Epoch(block_date.epoch()),
            self.block0_configuration
                .blockchain_configuration
                .slots_per_epoch
                .into(),
        )
    }

    pub fn to_remote(&self) -> RemoteJormungandr {
        let mut builder = RemoteJormungandrBuilder::new(self.alias.clone());
        builder.with_rest(self.rest_socket_addr);
        builder.build()
    }

    pub fn steal_temp_dir(&mut self) -> Option<TempDir> {
        self.temp_dir.take()
    }

    pub fn stop(mut self) {
        match self.child.kill() {
            Err(e) => println!("Could not kill {}: {}", self.alias, e),
            Ok(_) => {
                println!("Successfully killed {}", self.alias);
            }
        }
    }
}

impl Drop for JormungandrProcess {
    fn drop(&mut self) {
        // There's no kill like overkill
        let _ = self.child.kill();
        // FIXME: These should be better done in a test harness
        self.child.wait().unwrap();

        utils::persist_dir_on_panic(
            self.temp_dir.take(),
            vec![("node.log", &self.log_content())],
        );
    }
}

impl SyncNode for JormungandrProcess {
    fn alias(&self) -> &str {
        self.alias()
    }

    fn last_block_height(&self) -> u32 {
        let docs = self.rest().stats().unwrap();
        docs.stats
            .expect("no stats object in response")
            .last_block_height
            .expect("last_block_height field is missing")
            .parse()
            .unwrap()
    }

    fn log_stats(&self) {
        println!("{:?}", self.rest().stats());
    }

    fn tip(&self) -> Hash {
        self.rest().tip().expect("cannot get tip from rest")
    }

    fn log_content(&self) -> String {
        self.logger.get_log_content()
    }

    fn get_lines_with_error_and_invalid(&self) -> Vec<String> {
        self.logger
            .get_lines_with_level(LogLevel::ERROR)
            .map(|x| x.to_string())
            .collect()
    }

    fn is_running(&self) -> bool {
        matches!(self.status(&StartupVerificationMode::Log), Status::Running)
    }
}
