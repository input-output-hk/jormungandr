#![allow(dead_code)]

use crate::{
    legacy::{LegacyNode, LegacySettings},
    scenario::ProgressBarMode,
    style, Context,
};
use chain_impl_mockchain::{
    block::Block,
    fragment::{Fragment, FragmentId},
    header::HeaderId,
    testing::TestGen,
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{
        EnclaveLeaderId, FragmentLog, LeadershipLog, Log, LogEntry, LogOutput, NodeState,
        NodeStatsDto, PeerRecord, PeerStats, VotePlanStatus,
    },
};
pub use jormungandr_testing_utils::testing::{
    network_builder::{
        LeadershipMode, NodeAlias, NodeBlock0, NodeSetting, PersistenceMode, Settings,
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

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
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
    #[error("node stats in an invalid format")]
    InvalidNodeStats(#[source] RestError),
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

/// send query to a running node
pub struct NodeController {
    alias: NodeAlias,
    rest_client: JormungandrRest,
    grpc_client: JormungandrClient,
    settings: NodeSetting,
    progress_bar: ProgressBarController,
    status: Arc<Mutex<Status>>,
    process_id: u32,
    logger: JormungandrLogger,
}

/// Node is going to be used by the `Controller` to monitor the node process
///
/// To send queries to the Node, use the `NodeController`
pub struct Node {
    alias: NodeAlias,

    #[allow(unused)]
    dir: PathBuf,

    process: Child,

    progress_bar: ProgressBarController,
    node_settings: NodeSetting,
    status: Arc<Mutex<Status>>,
}

const NODE_CONFIG: &str = "node_config.yaml";
const NODE_SECRET: &str = "node_secret.yaml";
const NODE_STORAGE: &str = "storage.db";
const NODE_LOG: &str = "node.log";

impl NodeController {
    pub fn alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn status(&self) -> Status {
        *self.status.lock().unwrap()
    }

    pub fn check_running(&self) -> bool {
        self.status() == Status::Running
    }

    fn path(&self, path: &str) -> String {
        format!("{}/{}", self.base_url(), path)
    }

    pub fn address(&self) -> poldercast::Address {
        self.settings.config.p2p.public_address.clone()
    }

    pub fn explorer(&self) -> Explorer {
        Explorer::new(self.settings.config.rest.listen.to_string())
    }

    pub fn as_named_process(&self) -> NamedProcess {
        NamedProcess::new(self.alias().to_string(), self.process_id as usize)
    }

    fn get(&self, path: &str) -> Result<reqwest::blocking::Response> {
        self.progress_bar.log_info(format!("GET '{}'", path));

        match reqwest::blocking::get(&format!("{}/{}", self.base_url(), path)) {
            Err(err) => {
                self.progress_bar
                    .log_err(format!("Failed to send request {}", &err));
                Err(err.into())
            }
            Ok(r) => Ok(r),
        }
    }

    fn base_url(&self) -> String {
        format!("http://{}/api/v0", self.settings.config.rest.listen.clone())
    }

    pub fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck> {
        use chain_core::property::Fragment as _;

        let fragment_id = fragment.id();
        let result = self.rest_client.send_fragment(fragment);

        self.progress_bar
            .log_info(format!("Fragment '{}' sent", fragment_id));

        if let Err(err) = result {
            self.progress_bar
                .log_err(format!("Fragment ({}) fail to send: {}", fragment_id, err,));
        }

        Ok(MemPoolCheck::new(fragment_id))
    }

    pub fn log(&self, info: &str) {
        self.progress_bar.log_info(info);
    }

    pub fn tip(&self) -> Result<Hash> {
        let hash = self.rest_client.tip()?;
        self.progress_bar.log_info(format!("tip '{}'", hash));
        Ok(hash)
    }

    pub fn blocks_to_tip(&self, from: HeaderId) -> Result<Vec<Block>> {
        self.grpc_client
            .pull_blocks_to_tip(from)
            .map_err(Error::InvalidGrpcCall)
    }

    pub fn network_stats(&self) -> Result<Vec<PeerStats>> {
        let network_stats = self.rest_client.network_stats()?;
        self.progress_bar
            .log_info(format!("network_stats: '{:?}'", network_stats));
        Ok(network_stats)
    }

    pub fn p2p_quarantined(&self) -> Result<Vec<PeerRecord>> {
        let p2p_quarantined = self.rest_client.p2p_quarantined()?;
        self.progress_bar
            .log_info(format!("network/p2p_quarantined: {:?}", p2p_quarantined));
        Ok(p2p_quarantined)
    }

    pub fn p2p_non_public(&self) -> Result<Vec<PeerRecord>> {
        let non_public = self.rest_client.p2p_non_public()?;
        self.progress_bar
            .log_info(format!("network/p2p/non_public: {:?}", non_public));
        Ok(non_public)
    }

    pub fn p2p_available(&self) -> Result<Vec<PeerRecord>> {
        let p2p_available = self.rest_client.p2p_available()?;
        self.progress_bar
            .log_info(format!("network/p2p/available: {:?}", p2p_available));
        Ok(p2p_available)
    }

    pub fn p2p_view(&self) -> Result<Vec<String>> {
        let p2p_view = self.rest_client.p2p_view()?;
        self.progress_bar
            .log_info(format!("network/p2p/view: {:?}", p2p_view));
        Ok(p2p_view)
    }

    pub fn all_blocks_hashes(&self) -> Result<Vec<HeaderId>> {
        let genesis_hash = self
            .genesis_block_hash()
            .expect("Cannot download genesis hash");
        self.blocks_hashes_to_tip(genesis_hash)
    }

    pub fn blocks_hashes_to_tip(&self, from: HeaderId) -> Result<Vec<HeaderId>> {
        Ok(self
            .blocks_to_tip(from)
            .unwrap()
            .iter()
            .map(|x| x.header.hash())
            .collect())
    }

    pub fn genesis_block_hash(&self) -> Result<HeaderId> {
        Ok(self.grpc_client.get_genesis_block_hash())
    }

    pub fn block(&self, header_hash: &HeaderId) -> Result<Block> {
        use chain_core::mempack::{ReadBuf, Readable as _};

        let mut resp = self.get(&format!("block/{}", header_hash))?;
        let mut bytes = Vec::new();
        resp.copy_to(&mut bytes)?;
        let block = Block::read(&mut ReadBuf::from(&bytes)).map_err(Error::InvalidBlock)?;

        self.progress_bar.log_info(format!(
            "block{} ({}) '{}'",
            block.header.chain_length(),
            block.header.block_date(),
            header_hash,
        ));

        Ok(block)
    }

    pub fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>> {
        let logs = self.rest_client.fragment_logs()?;
        self.progress_bar
            .log_info(format!("fragment logs ({})", logs.len()));
        Ok(logs)
    }

    pub fn leaders(&self) -> Result<Vec<EnclaveLeaderId>> {
        let leaders = self.rest_client.leaders()?;
        self.progress_bar
            .log_info(format!("leaders ids ({})", leaders.len()));
        Ok(leaders)
    }

    pub fn promote(&self) -> Result<EnclaveLeaderId> {
        let path = "leaders";
        let secrets = self.settings.secrets();
        self.progress_bar.log_info(format!("POST '{}'", &path));
        let response = reqwest::blocking::Client::new()
            .post(&self.path(path))
            .json(&secrets)
            .send()?;

        self.progress_bar
            .log_info(format!("Leader promotion for '{}' sent", self.alias()));

        let res = response.error_for_status_ref();
        if let Err(err) = res {
            self.progress_bar.log_err(format!(
                "Leader promotion for '{}' fail to sent: {}",
                self.alias(),
                err,
            ));
        }

        let leader_id: EnclaveLeaderId = response.json()?;
        Ok(leader_id)
    }

    pub fn demote(&self, leader_id: u32) -> Result<()> {
        let path = format!("leaders/{}", leader_id);
        self.progress_bar.log_info(format!("DELETE '{}'", &path));
        let response = reqwest::blocking::Client::new()
            .delete(&self.path(&path))
            .send()?;

        self.progress_bar
            .log_info(format!("Leader demote for '{}' sent", self.alias()));

        let res = response.error_for_status_ref();
        if let Err(err) = res {
            self.progress_bar.log_err(format!(
                "Leader demote for '{}' fail to sent: {}",
                self.alias(),
                err,
            ));
        }
        Ok(())
    }

    pub fn stats(&self) -> Result<NodeStatsDto> {
        self.rest_client.stats().map_err(Error::InvalidNodeStats)
    }

    pub fn vote_plans(&self) -> Result<Vec<VotePlanStatus>> {
        Ok(self.rest_client.vote_plan_statuses()?)
    }

    pub fn log_stats(&self) {
        self.progress_bar
            .log_info(format!("node stats ({:?})", self.stats()));
    }

    pub fn leadership_log(&self) -> Result<Vec<LeadershipLog>> {
        Ok(self.rest_client.leaders_log()?)
    }

    pub fn log_leadership_log(&self) {
        self.progress_bar
            .log_info(format!("{:?}", self.leadership_log().unwrap()));
    }

    pub fn wait_for_bootstrap(&self) -> Result<()> {
        let max_try = 20;
        let sleep = Duration::from_secs(8);
        for _ in 0..max_try {
            let stats = self.stats();
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
            alias: self.alias().to_string(),
            duration: Duration::from_secs(sleep.as_secs() * max_try),
            logs: self.logger().get_lines_as_string(),
        })
    }

    pub fn wait_for_shutdown(&self) -> Result<()> {
        let max_try = 2;
        let sleep = Duration::from_secs(2);
        for _ in 0..max_try {
            if self.stats().is_err() && self.ports_are_opened() {
                return Ok(());
            };
            std::thread::sleep(sleep);
        }
        Err(Error::NodeFailedToShutdown {
            alias: self.alias().to_string(),
            message: format!(
                "node is still up after {} s from sending shutdown request",
                sleep.as_secs()
            ),
            logs: self.logger().get_lines_as_string(),
        })
    }

    fn ports_are_opened(&self) -> bool {
        self.port_opened(self.settings.config.rest.listen.port())
            && self.port_opened(
                self.settings
                    .config
                    .p2p
                    .get_listen_address()
                    .to_socket_addr()
                    .unwrap()
                    .port(),
            )
    }

    fn port_opened(&self, port: u16) -> bool {
        use std::net::TcpListener;
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    pub fn is_up(&self) -> bool {
        let stats = self.stats();
        match stats {
            Ok(stats) => stats.state == NodeState::Running,
            Err(_) => false,
        }
    }

    pub fn shutdown(&self) -> Result<()> {
        let result = self.get("shutdown")?.text()?;

        if result.is_empty() {
            self.progress_bar.log_info("shuting down");
            self.wait_for_shutdown()
        } else {
            Err(Error::NodeFailedToShutdown {
                alias: self.alias().to_string(),
                message: result,
                logs: self.logger().get_lines_as_string(),
            })
        }
    }

    pub fn progress_bar(&self) -> &ProgressBarController {
        &self.progress_bar
    }

    pub fn logger(&self) -> &JormungandrLogger {
        &self.logger
    }

    pub fn log_content(&self) -> String {
        self.logger().get_log_content()
    }
}

impl Node {
    pub fn alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn controller(mut self) -> NodeController {
        let p2p_address = format!("{}", self.node_settings.config().p2p.get_listen_address());
        let rest_uri = uri_from_socket_addr(self.node_settings.config().rest.listen);

        NodeController {
            alias: self.alias().clone(),
            logger: JormungandrLogger::new(self.process.stdout.take().unwrap()),
            grpc_client: JormungandrClient::from_address(&p2p_address)
                .expect("cannot setup grpc client"),
            rest_client: JormungandrRest::new(rest_uri),
            settings: self.node_settings,
            status: self.status,
            progress_bar: self.progress_bar,
            process_id: self.process.id(),
        }
    }

    pub fn spawn<'a, R: RngCore>(
        context: &'a Context<R>,
        node_settings: &'a mut NodeSetting,
    ) -> SpawnBuilder<'a, R, Node> {
        SpawnBuilder::new(&context, node_settings)
    }

    pub fn progress_bar(&self) -> &ProgressBarController {
        &self.progress_bar
    }

    pub fn capture_logs(&mut self) {
        let stderr = self.process.stderr.take().unwrap();
        let reader = BufReader::new(stderr);
        for line_result in reader.lines() {
            let line = line_result.expect("failed to read a line from log output");
            self.progress_bar.log_info(&line);
        }
    }

    pub fn wait(&mut self) {
        match self.process.wait() {
            Err(err) => {
                self.progress_bar.log_err(&err);
                self.progress_bar_failure();
                self.set_status(Status::Failure);
            }
            Ok(status) => {
                if status.success() {
                    self.progress_bar_success();
                } else {
                    self.progress_bar.log_err(&status);
                    self.progress_bar_failure()
                }
                self.set_status(Status::Exit(status));
            }
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
            self.node_settings.config().rest.listen,
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
    block0: NodeBlock0,
    working_dir: PathBuf,
    peristence_mode: PersistenceMode,
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
            block0: NodeBlock0::Hash(TestGen::hash()),
            working_dir: PathBuf::new(),
            peristence_mode: PersistenceMode::Persistent,
            phantom_data: PhantomData,
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

    pub fn block0(&mut self, block0: NodeBlock0) -> &mut Self {
        self.block0 = block0;
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
            self.node_settings.config(),
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
            self.node_settings.secrets(),
        )
        .map_err(|e| Error::CannotWriteYamlFile {
            path: config_secret.as_ref().to_path_buf(),
            cause: e,
        })
    }

    fn apply_persistence_setting(&mut self, dir: &PathBuf) {
        if self.peristence_mode == PersistenceMode::Persistent {
            let path_to_storage = dir.join(NODE_STORAGE);
            self.node_settings.config.storage = Some(path_to_storage);
        }
    }

    fn set_log_level(&mut self, log_file: &PathBuf) {
        let format = "plain";
        let level = self.context.log_level();
        self.node_settings.config.log = Some(Log(vec![
            LogEntry {
                format: format.to_string(),
                level: level.to_string(),
                output: LogOutput::Stderr,
            },
            LogEntry {
                format: format.to_string(),
                level,
                output: LogOutput::File(log_file.to_path_buf()),
            },
        ]));
    }

    pub fn command<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        config_file: P,
        config_secret: Q,
    ) -> Command {
        let mut command = Command::new(self.jormungandr.clone());

        command.arg("--config");
        command.arg(config_file.as_ref());

        match &self.block0 {
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
        let dir = self.working_dir.join(self.alias.to_owned());
        std::fs::DirBuilder::new().recursive(true).create(&dir)?;

        let config_file = dir.join(NODE_CONFIG);
        let config_secret = dir.join(NODE_SECRET);
        let log_file = dir.join(NODE_LOG);

        self.set_log_level(&log_file);
        self.apply_persistence_setting(&dir);
        self.write_config_file(&config_file)?;
        self.write_secret_file(&config_secret)?;

        let mut command = self.command(config_file, config_secret);
        let process = command.spawn().map_err(Error::CannotSpawnNode)?;

        let progress_bar = ProgressBarController::new(
            self.progress_bar,
            format!("{}@{}", self.alias, self.node_settings.config().rest.listen),
            self.context.progress_bar_mode(),
        );

        let node = Node {
            alias: self.alias.clone(),
            dir,
            process,
            progress_bar,
            node_settings: self.node_settings.clone(),
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
        let log_file = dir.join(NODE_LOG);

        self.set_log_level(&log_file);
        self.apply_persistence_setting(&dir);
        self.write_config_file(&config_file)?;
        self.write_secret_file(&config_secret)?;

        let mut command = self.command(config_file, config_secret);
        let process = command.spawn().map_err(Error::CannotSpawnNode)?;

        let progress_bar = ProgressBarController::new(
            self.progress_bar,
            format!("{}@{}", self.alias, self.node_settings.config().rest.listen),
            self.context.progress_bar_mode(),
        );

        let legacy_settngs = LegacySettings::from_settings(self.node_settings.clone(), version);

        let node = LegacyNode {
            alias: self.alias.clone(),
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
