#![allow(dead_code)]

use crate::{
    legacy::{LegacyNode, LegacySettings},
    scenario::ProgressBarMode,
    style, Context,
};
use chain_impl_mockchain::{fragment::FragmentId, testing::TestGen};
use jormungandr_lib::interfaces::Block0Configuration;
use jormungandr_lib::{crypto::hash::Hash, interfaces::NodeState, multiaddr};
use jormungandr_testing_utils::testing::jormungandr::JormungandrProcess;
use jormungandr_testing_utils::testing::jormungandr::StartupError;
pub use jormungandr_testing_utils::testing::{
    network::{
        FaketimeConfig, LeadershipMode, NodeAlias, NodeBlock0, NodeSetting, PersistenceMode,
        Settings,
    },
    node::{
        grpc::{client::MockClientError, JormungandrClient},
        uri_from_socket_addr, JormungandrLogger, JormungandrRest, RestError,
    },
    FragmentNode, MemPoolCheck, NamedProcess,
};
use jormungandr_testing_utils::{testing::node::Explorer, Version};

use indicatif::ProgressBar;
use rand_core::RngCore;
use std::net::SocketAddr;

use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(custom_debug::Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    BlockFormatError(#[from] chain_core::mempack::ReadError),
    #[error(transparent)]
    RestError(#[from] RestError),
    #[error(transparent)]
    SerializationError(#[from] yaml_rust::scanner::ScanError),
    #[error(transparent)]
    GrpcError(#[from] MockClientError),
    #[error("cannot create file {path}")]
    CannotCreateFile {
        path: PathBuf,
        #[source]
        cause: io::Error,
    },
    #[error("cannot write YAML into {path}")]
    CannotWriteYamlFile {
        path: PathBuf,
        #[source]
        cause: serde_yaml::Error,
    },
    #[error("cannot spawn the node")]
    CannotSpawnNode(#[source] io::Error),
    // FIXME: duplicate of GrpcError?
    #[error("invalid grpc call")]
    InvalidGrpcCall(#[source] MockClientError),
    #[error("invalid header id")]
    InvalidHeaderId(#[source] chain_crypto::hash::Error),
    #[error("invalid block")]
    InvalidBlock(#[source] chain_core::mempack::ReadError),
    #[error("fragment logs in an invalid format")]
    InvalidFragmentLogs(#[source] serde_json::Error),
    #[error("rest error")]
    Rest(#[source] RestError),
    #[error("network stats in an invalid format")]
    InvalidNetworkStats(#[source] serde_json::Error),
    #[error("leaders ids in an invalid format")]
    InvalidEnclaveLeaderIds(#[source] serde_json::Error),
    #[error("node '{alias}' failed to start after {} s. Logs: {}", .duration.as_secs(), logs.join("\n"))]
    NodeFailedToBootstrap {
        alias: String,
        duration: Duration,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("node '{alias}' failed to shutdown, message: {message}")]
    NodeFailedToShutdown {
        alias: String,
        message: String,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("fragment '{fragment_id}' not in the mempool of the node '{alias}'")]
    FragmentNotInMemPoolLogs {
        alias: String,
        fragment_id: FragmentId,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("fragment '{fragment_id}' is pending for too long ({} s) for node '{alias}'", .duration.as_secs())]
    FragmentIsPendingForTooLong {
        fragment_id: FragmentId,
        duration: Duration,
        alias: String,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error(transparent)]
    Startup(#[from] StartupError),
}

impl Error {
    pub fn logs(&self) -> impl Iterator<Item = &str> {
        use self::Error::*;
        let maybe_logs = match self {
            NodeFailedToBootstrap { logs, .. }
            | NodeFailedToShutdown { logs, .. }
            | FragmentNotInMemPoolLogs { logs, .. }
            | FragmentIsPendingForTooLong { logs, .. } => Some(logs),
            _ => None,
        };
        maybe_logs
            .into_iter()
            .map(|logs| logs.iter())
            .flatten()
            .map(String::as_str)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Status {
    Running,
    Failure,
    Exit(ExitStatus),
}

#[derive(Clone)]
pub struct ProgressBarController {
    progress_bar: ProgressBar,
    prefix: String,
    logging_mode: ProgressBarMode,
}

/// Node is going to be used by the `Controller` to monitor the node process
pub struct Node {
    dir: PathBuf,
    process: JormungandrProcess,

    progress_bar: ProgressBarController,
    status: Arc<Mutex<Status>>,
}

const NODE_CONFIG: &str = "node_config.yaml";
const NODE_SECRET: &str = "node_secret.yaml";
const NODE_TOPOLOGY_KEY: &str = "node_topology_key";
const NODE_STORAGE: &str = "storage.db";

impl Node {
    pub fn alias(&self) -> NodeAlias {
        self.process.alias()
    }

    pub fn status(&self) -> Status {
        *self.status.lock().unwrap()
    }

    pub fn check_running(&self) -> bool {
        self.status() == Status::Running
    }

    pub fn address(&self) -> SocketAddr {
        multiaddr::to_tcp_socket_addr(&self.process.p2p_public_address()).unwrap()
    }

    pub fn explorer(&self) -> Explorer {
        self.process.explorer()
    }

    pub fn as_named_process(&self) -> NamedProcess {
        NamedProcess::new(self.alias(), self.process.process_id() as usize)
    }

    pub fn log(&self, info: &str) {
        self.progress_bar.log_info(info);
    }

    pub fn tip(&self) -> Result<Hash> {
        let hash = self.rest().tip()?;
        self.progress_bar.log_info(format!("tip '{}'", hash));
        Ok(hash)
    }

    pub fn rest(&self) -> JormungandrRest {
        self.process.rest()
    }

    pub fn grpc(&self) -> JormungandrClient {
        self.process.grpc()
    }
    pub fn log_stats(&self) {
        self.progress_bar
            .log_info(format!("node stats ({:?})", self.rest().stats()));
    }

    pub fn log_leadership_log(&self) {
        self.progress_bar
            .log_info(format!("{:?}", self.rest().leaders_log().unwrap()));
    }

    pub fn wait_for_bootstrap(&self) -> Result<()> {
        let max_try = 20;
        let sleep = Duration::from_secs(8);
        for _ in 0..max_try {
            let stats = self.rest().stats();
            match stats {
                Ok(stats) => {
                    if stats.state == NodeState::Running {
                        self.log_stats();
                        return Ok(());
                    }
                }
                Err(err) => self
                    .progress_bar
                    .log_info(format!("node stats failure({:?})", err)),
            };
            std::thread::sleep(sleep);
        }
        Err(Error::NodeFailedToBootstrap {
            alias: self.alias(),
            duration: Duration::from_secs(sleep.as_secs() * max_try),
            logs: self.logger().get_lines_as_string(),
        })
    }

    pub fn wait_for_shutdown(&self) -> Result<()> {
        let max_try = 2;
        let sleep = Duration::from_secs(2);
        for _ in 0..max_try {
            if self.rest().stats().is_err() && self.ports_are_opened() {
                return Ok(());
            };
            std::thread::sleep(sleep);
        }
        Err(Error::NodeFailedToShutdown {
            alias: self.alias(),
            message: format!(
                "node is still up after {} s from sending shutdown request",
                sleep.as_secs()
            ),
            logs: self.logger().get_lines_as_string(),
        })
    }

    fn ports_are_opened(&self) -> bool {
        self.port_opened(self.process.rest_address().port())
            && self.port_opened(self.process.p2p_listen_addr().port())
    }

    fn port_opened(&self, port: u16) -> bool {
        use std::net::TcpListener;
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    pub fn is_up(&self) -> bool {
        let stats = self.rest().stats();
        match stats {
            Ok(stats) => stats.state == NodeState::Running,
            Err(_) => false,
        }
    }

    pub fn shutdown(&self) -> Result<()> {
        let message = self.rest().shutdown()?;

        if message.is_empty() {
            self.progress_bar.log_info("shuting down");
            self.wait_for_shutdown()
        } else {
            Err(Error::NodeFailedToShutdown {
                alias: self.alias(),
                message,
                logs: self.logger().get_lines_as_string(),
            })
        }
    }

    pub fn logger(&self) -> &JormungandrLogger {
        &self.process.logger
    }

    pub fn log_content(&self) -> String {
        self.logger().get_log_content()
    }

    pub fn progress_bar(&self) -> &ProgressBarController {
        &self.progress_bar
    }

    pub fn capture_logs(&mut self) {
        let stderr = self.process.child.stderr.take().unwrap();
        let reader = BufReader::new(stderr);
        for line_result in reader.lines() {
            let line = line_result.expect("failed to read a line from log output");
            self.progress_bar.log_info(&line);
        }
    }

    fn progress_bar_start(&self) {
        self.progress_bar.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.green} {wide_msg}")
                .tick_chars(style::TICKER),
        );
        self.progress_bar.enable_steady_tick(100);
        self.progress_bar.set_message(&format!(
            "{} {} ... [{}]",
            *style::icons::jormungandr,
            style::binary.apply_to(self.alias()),
            self.process.rest_address(),
        ));
    }

    fn progress_bar_failure(&self) {
        self.progress_bar.finish_with_message(&format!(
            "{} {} {}",
            *style::icons::jormungandr,
            style::binary.apply_to(self.alias()),
            style::error.apply_to(*style::icons::failure)
        ));
    }

    fn progress_bar_success(&self) {
        self.progress_bar.finish_with_message(&format!(
            "{} {} {}",
            *style::icons::jormungandr,
            style::binary.apply_to(self.alias()),
            style::success.apply_to(*style::icons::success)
        ));
    }

    fn set_status(&self, status: Status) {
        *self.status.lock().unwrap() = status
    }
}

use std::fmt::Display;

impl ProgressBarController {
    pub fn new(progress_bar: ProgressBar, prefix: String, logging_mode: ProgressBarMode) -> Self {
        ProgressBarController {
            progress_bar,
            prefix,
            logging_mode,
        }
    }

    pub fn log_info<M>(&self, msg: M)
    where
        M: Display,
    {
        self.log(style::info.apply_to("INFO "), msg)
    }

    pub fn log_err<M>(&self, msg: M)
    where
        M: Display,
    {
        self.log(style::error.apply_to("ERROR"), style::error.apply_to(msg))
    }

    pub fn log<L, M>(&self, lvl: L, msg: M)
    where
        L: Display,
        M: Display,
    {
        match self.logging_mode {
            ProgressBarMode::Standard => {
                println!("[{}][{}]: {}", lvl, &self.prefix, msg);
            }
            ProgressBarMode::Monitor => {
                self.progress_bar.println(format!(
                    "[{}][{}{}]: {}",
                    lvl,
                    *style::icons::jormungandr,
                    style::binary.apply_to(&self.prefix),
                    msg,
                ));
            }
            ProgressBarMode::None => (),
        }
    }
}

impl std::ops::Deref for ProgressBarController {
    type Target = ProgressBar;
    fn deref(&self) -> &Self::Target {
        &self.progress_bar
    }
}

use std::marker::PhantomData;

pub struct SpawnBuilder<'a, R: RngCore, N> {
    jormungandr: PathBuf,
    context: &'a Context<R>,
    progress_bar: ProgressBar,
    alias: String,
    node_settings: &'a mut NodeSetting,
    block0: Option<Block0Configuration>,
    block0_setting: NodeBlock0,
    working_dir: PathBuf,
    peristence_mode: PersistenceMode,
    faketime: Option<FaketimeConfig>,
    phantom_data: PhantomData<N>,
}

impl<'a, R: RngCore, N> SpawnBuilder<'a, R, N> {
    pub fn new(context: &'a Context<R>, node_settings: &'a mut NodeSetting) -> Self {
        Self {
            jormungandr: PathBuf::new(),
            context,
            progress_bar: ProgressBar::hidden(),
            alias: "".to_owned(),
            node_settings,
            block0_setting: NodeBlock0::Hash(TestGen::hash()),
            block0: None,
            working_dir: PathBuf::new(),
            peristence_mode: PersistenceMode::Persistent,
            phantom_data: PhantomData,
            faketime: None,
        }
    }

    pub fn path_to_jormungandr<P: AsRef<Path>>(&mut self, path_to_jormungandr: P) -> &mut Self {
        self.jormungandr = path_to_jormungandr.as_ref().to_path_buf();
        self
    }
    pub fn progress_bar(&mut self, progress_bar: ProgressBar) -> &mut Self {
        self.progress_bar = progress_bar;
        self
    }

    pub fn alias<S: Into<String>>(&mut self, alias: S) -> &mut Self {
        self.alias = alias.into();
        self
    }

    pub fn faketime(&mut self, faketime: FaketimeConfig) -> &mut Self {
        self.faketime = Some(faketime);
        self
    }

    pub fn block0_setting(&mut self, block0: NodeBlock0) -> &mut Self {
        self.block0_setting = block0;
        self
    }

    pub fn block0(&mut self, block0: Block0Configuration) -> &mut Self {
        self.block0 = Some(block0);
        self
    }

    pub fn working_dir<P: AsRef<Path>>(&mut self, working_dir: P) -> &mut Self {
        self.working_dir = working_dir.as_ref().to_path_buf();
        self
    }

    pub fn peristence_mode(&mut self, persistence_mode: PersistenceMode) -> &mut Self {
        self.peristence_mode = persistence_mode;
        self
    }

    fn write_config_file<P: AsRef<Path>>(&self, config_file: P) -> Result<()> {
        serde_yaml::to_writer(
            std::fs::File::create(config_file.as_ref()).map_err(|e| Error::CannotCreateFile {
                path: config_file.as_ref().to_path_buf(),
                cause: e,
            })?,
            &self.node_settings.config,
        )
        .map_err(|e| Error::CannotWriteYamlFile {
            path: config_file.as_ref().to_path_buf(),
            cause: e,
        })
    }

    fn write_secret_file<P: AsRef<Path>>(&self, config_secret: P) -> Result<()> {
        serde_yaml::to_writer(
            std::fs::File::create(&config_secret).map_err(|e| Error::CannotCreateFile {
                path: config_secret.as_ref().to_path_buf(),
                cause: e,
            })?,
            &self.node_settings.secret,
        )
        .map_err(|e| Error::CannotWriteYamlFile {
            path: config_secret.as_ref().to_path_buf(),
            cause: e,
        })
    }

    fn write_topology_file<P: AsRef<Path>>(&self, key_file: P) -> Result<()> {
        Ok(std::fs::write(
            key_file.as_ref(),
            self.node_settings.topology_secret.to_bech32_str(),
        )?)
    }

    fn apply_persistence_setting(&mut self, dir: &Path) {
        if self.peristence_mode == PersistenceMode::Persistent {
            let path_to_storage = dir.join(NODE_STORAGE);
            self.node_settings.config.storage = Some(path_to_storage);
        }
    }

    pub fn command<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        config_file: P,
        config_secret: Q,
    ) -> Command {
        let mut command = if let Some(faketime) = &self.faketime {
            let mut cmd = Command::new("faketime");
            cmd.args(&["-f", &format!("{:+}s", faketime.offset)]);
            cmd.arg(self.jormungandr.clone());
            cmd
        } else {
            Command::new(self.jormungandr.clone())
        };

        command.arg("--config");
        command.arg(config_file.as_ref());

        match &self.block0_setting {
            NodeBlock0::File(path) => {
                command.arg("--genesis-block");
                command.arg(&path);
                command.arg("--secret");
                command.arg(config_secret.as_ref());
            }
            NodeBlock0::Hash(hash) => {
                command.args(&["--genesis-block-hash", &hash.to_string()]);
            }
        }

        command.stderr(Stdio::piped());
        command.stdout(Stdio::piped());
        command
    }
}

impl<'a, R: RngCore> SpawnBuilder<'a, R, Node> {
    pub fn build(mut self) -> Result<Node> {
        let dir = self.working_dir.clone();
        std::fs::DirBuilder::new().recursive(true).create(&dir)?;

        let config_file = dir.join(NODE_CONFIG);
        let config_secret = dir.join(NODE_SECRET);
        let topology_key_file = dir.join(NODE_TOPOLOGY_KEY);

        self.apply_persistence_setting(&dir);

        self.node_settings.config.p2p.node_key_file = Some(topology_key_file.clone());
        self.write_topology_file(&topology_key_file)?;
        self.write_config_file(&config_file)?;
        self.write_secret_file(&config_secret)?;

        let mut command = self.command(config_file, config_secret);
        let process = command.spawn().map_err(Error::CannotSpawnNode)?;

        let progress_bar = ProgressBarController::new(
            self.progress_bar,
            format!("{}@{}", self.alias, self.node_settings.config.rest.listen),
            self.context.progress_bar_mode(),
        );

        let process = JormungandrProcess::new(
            process,
            &self.node_settings.config,
            self.block0.unwrap(),
            None,
            self.alias.clone(),
        )?;

        let node = Node {
            dir,
            process,
            progress_bar,
            status: Arc::new(Mutex::new(Status::Running)),
        };

        node.progress_bar_start();
        node.progress_bar
            .log_info(&format!("{} bootstrapping: {:?}", self.alias, command));
        Ok(node)
    }
}

impl<'a, R: RngCore> SpawnBuilder<'a, R, LegacyNode> {
    pub fn build(mut self, version: &Version) -> Result<LegacyNode> {
        let dir = self.working_dir.join(self.alias.to_owned());
        std::fs::DirBuilder::new().recursive(true).create(&dir)?;

        let config_file = dir.join(NODE_CONFIG);
        let config_secret = dir.join(NODE_SECRET);

        self.apply_persistence_setting(&dir);
        self.write_config_file(&config_file)?;
        self.write_secret_file(&config_secret)?;

        let mut command = self.command(config_file, config_secret);
        let process = command.spawn().map_err(Error::CannotSpawnNode)?;

        let progress_bar = ProgressBarController::new(
            self.progress_bar,
            format!("{}@{}", self.alias, self.node_settings.config.rest.listen),
            self.context.progress_bar_mode(),
        );

        let legacy_settngs = LegacySettings::from_settings(self.node_settings.clone(), version);

        let process = JormungandrProcess::new(
            process,
            legacy_settngs.config(),
            self.block0.unwrap(),
            None,
            self.alias.clone(),
        )?;

        let node = LegacyNode {
            dir,
            process,
            progress_bar,
            node_settings: legacy_settngs,
            status: Arc::new(Mutex::new(Status::Running)),
        };

        node.progress_bar_start();
        node.progress_bar()
            .log_info(&format!("{} bootstrapping: {:?}", self.alias, command));
        Ok(node)
    }
}
