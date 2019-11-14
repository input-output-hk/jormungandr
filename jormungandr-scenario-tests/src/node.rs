use crate::{scenario::settings::NodeSetting, style, Context, NodeAlias};
use bawawa::{Control, Process};
use chain_impl_mockchain::{
    block::Block,
    fragment::{Fragment, FragmentId},
    header::HeaderId,
};
use indicatif::ProgressBar;
use jormungandr_integration_tests::mock::{client::JormungandrClient, read_into};
use jormungandr_lib::interfaces::{FragmentLog, FragmentStatus, Stats};
#[macro_use]
use jormungandr_integration_tests::response_to_vec;
use rand_core::RngCore;
use std::{
    collections::HashMap,
    path::PathBuf,
    process::ExitStatus,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::prelude::*;

error_chain! {
    foreign_links {
        Io(std::io::Error);
        Reqwest(reqwest::Error);
        BlockFormatError(chain_core::mempack::ReadError);
    }

    errors {
        CannotCreateTemporaryDirectory {
            description("Cannot create a temporary directory")
        }

        CannotSpawnNode {
            description("Cannot spawn the node"),
        }

        InvalidHeaderId {
            description("Invalid header id"),
        }

        InvalidBlock {
            description("Invalid block"),
        }
        InvalidFragmentLogs {
            description("Fragment logs in an invalid format")
        }
        InvalidNodeStats {
            description("Node stats in an invalid format")
        }

        NodeStopped (status: Status) {
            description("the node is no longer running"),
        }

        FragmentNoInMemPoolLogs (fragment_id: FragmentId) {
            description("cannot find fragment in mempool logs"),
            display("fragment '{}' not in the mempool of the node", fragment_id),
        }
    }
}

pub struct MemPoolCheck {
    fragment_id: FragmentId,
}

pub enum NodeBlock0 {
    Hash(HeaderId),
    File(PathBuf),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LeadershipMode {
    Leader,
    Passive,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PersistenceMode {
    Persistent,
    InMemory,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Status {
    Running,
    Failure,
    Exit(ExitStatus),
}

#[derive(Clone)]
struct ProgressBarController {
    progress_bar: ProgressBar,
    prefix: String,
}

/// send query to a running node
#[derive(Clone)]
pub struct NodeController {
    alias: NodeAlias,
    grpc_client: JormungandrClient,
    settings: NodeSetting,
    progress_bar: ProgressBarController,
    status: Arc<Mutex<Status>>,
}

/// Node is going to be used by the `Controller` to monitor the node process
///
/// To send queries to the Node, use the `NodeController`
pub struct Node {
    alias: NodeAlias,

    #[allow(unused)]
    dir: PathBuf,

    process: Process,

    progress_bar: ProgressBarController,
    node_settings: NodeSetting,
    status: Arc<Mutex<Status>>,
}

const NODE_CONFIG: &str = "node_config.yaml";
const NODE_SECRET: &str = "node_secret.yaml";
const NODE_STORAGE: &str = "storage.db";

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

    fn post(&self, path: &str, body: Vec<u8>) -> Result<reqwest::Response> {
        self.progress_bar.log_info(format!("POST '{}'", path));

        let client = reqwest::Client::new();
        let res = client
            .post(&format!("{}/{}", self.base_url(), path))
            .body(body)
            .send();

        match res {
            Err(err) => {
                self.progress_bar
                    .log_err(format!("Failed to send request {}", &err));
                Err(err.into())
            }
            Ok(r) => Ok(r),
        }
    }

    fn get(&self, path: &str) -> Result<reqwest::Response> {
        self.progress_bar.log_info(format!("GET '{}'", path));

        match reqwest::get(&format!("{}/{}", self.base_url(), path)) {
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
        use chain_core::property::Serialize as _;

        let raw = fragment.serialize_as_vec().unwrap();
        let fragment_id = fragment.id();

        let response = self.post("message", raw.clone())?;
        self.progress_bar
            .log_info(format!("Fragment '{}' sent", fragment_id,));

        let res = response.error_for_status_ref();
        if let Err(err) = res {
            self.progress_bar.log_err(format!(
                "Fragment '{}' ({}) fail to send: {}",
                hex::encode(&raw),
                fragment_id,
                err,
            ));
        }

        Ok(MemPoolCheck {
            fragment_id: fragment_id,
        })
    }

    pub fn tip(&self) -> Result<HeaderId> {
        let hash = self.get("tip")?.text()?;

        let hash = hash.parse().chain_err(|| ErrorKind::InvalidHeaderId)?;

        self.progress_bar.log_info(format!("tip '{}'", hash));

        Ok(hash)
    }

    pub fn blocks_to_tip(&self, from: HeaderId) -> Result<Vec<Block>> {
        let response = self.grpc_client.pull_blocks_to_tip(from);
        Ok(response_to_vec!(response))
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
        let block =
            Block::read(&mut ReadBuf::from(&bytes)).chain_err(|| ErrorKind::InvalidBlock)?;

        self.progress_bar.log_info(format!(
            "block{} ({}) '{}'",
            block.header.chain_length(),
            block.header.block_date(),
            header_hash,
        ));

        Ok(block)
    }

    pub fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>> {
        let logs = self.get("fragment/logs")?.text()?;

        let logs: Vec<FragmentLog> = if logs.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&logs).chain_err(|| ErrorKind::InvalidFragmentLogs)?
        };

        self.progress_bar
            .log_info(format!("fragment logs ({})", logs.len()));

        let logs = logs
            .into_iter()
            .map(|log| (log.fragment_id().clone().into_hash(), log))
            .collect();

        Ok(logs)
    }

    pub fn wait_fragment(&self, duration: Duration, check: MemPoolCheck) -> Result<FragmentStatus> {
        loop {
            let logs = self.fragment_logs()?;

            if let Some(log) = logs.get(&check.fragment_id) {
                use jormungandr_lib::interfaces::FragmentStatus::*;
                let status = log.status().clone();
                match log.status() {
                    Pending => {
                        self.progress_bar
                            .log_info(format!("Fragment '{}' is still pending", check.fragment_id));
                    }
                    Rejected { reason } => {
                        self.progress_bar.log_info(format!(
                            "Fragment '{}' rejected: {}",
                            check.fragment_id, reason
                        ));
                        return Ok(status);
                    }
                    InABlock { date, block } => {
                        self.progress_bar.log_info(format!(
                            "Fragment '{}' in block: {} ({})",
                            check.fragment_id, block, date
                        ));
                        return Ok(status);
                    }
                }
            } else {
                // bail!(ErrorKind::FragmentNoInMemPoolLogs(
                //    check.fragment_id.clone()
                //))
            }
            std::thread::sleep(duration);
        }
    }

    pub fn stats(&self) -> Result<Stats> {
        let stats = self.get("node/stats")?.text()?;
        let stats: Stats =
            serde_json::from_str(&stats).chain_err(|| ErrorKind::InvalidNodeStats)?;
        self.progress_bar
            .log_info(format!("node stats ({:?})", stats));
        Ok(stats)
    }

    pub fn wait_for_bootstrap(&self) {
        loop {
            let stats = self.stats();
            println!("{:?}", stats);
            if let Ok(stats) = stats {
                if stats.uptime > 0 {
                    return;
                }
            } else {
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    pub fn shutdown(&self) -> Result<bool> {
        let result = self.get("shutdown")?.text()?;

        if result == "Success" {
            self.progress_bar.log_info("shuting down");
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Node {
    pub fn alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn controller(&self) -> NodeController {
        let p2p_address = format!("{}", self.node_settings.config().p2p.public_address);

        NodeController {
            alias: self.alias().clone(),
            grpc_client: JormungandrClient::from_address(&p2p_address)
                .expect("cannot setup grpc client"),
            settings: self.node_settings.clone(),
            status: self.status.clone(),
            progress_bar: self.progress_bar.clone(),
        }
    }

    pub fn spawn<R: RngCore>(
        context: &Context<R>,
        progress_bar: ProgressBar,
        alias: &str,
        node_settings: &mut NodeSetting,
        block0: NodeBlock0,
        working_dir: &PathBuf,
        peristence_mode: PersistenceMode,
    ) -> Result<Self> {
        let mut command = context.jormungandr().clone();
        let dir = working_dir.join(alias);
        std::fs::DirBuilder::new().recursive(true).create(&dir)?;

        let progress_bar = ProgressBarController::new(
            progress_bar,
            format!("{}@{}", alias, node_settings.config().rest.listen),
        );

        let config_file = dir.join(NODE_CONFIG);
        let config_secret = dir.join(NODE_SECRET);

        if peristence_mode == PersistenceMode::Persistent {
            let path_to_storage = dir.join(NODE_STORAGE);
            node_settings.config.storage = Some(path_to_storage);
        }

        serde_yaml::to_writer(
            std::fs::File::create(&config_file)
                .chain_err(|| format!("Cannot create file {:?}", config_file))?,
            node_settings.config(),
        )
        .chain_err(|| format!("cannot write in {:?}", config_file))?;

        serde_yaml::to_writer(
            std::fs::File::create(&config_secret)
                .chain_err(|| format!("Cannot create file {:?}", config_secret))?,
            node_settings.secrets(),
        )
        .chain_err(|| format!("cannot write in {:?}", config_secret))?;

        command.arguments(&[
            "--config",
            &config_file.display().to_string(),
            "--secret",
            &config_secret.display().to_string(),
            "--log-level=warn",
        ]);

        match block0 {
            NodeBlock0::File(path) => {
                command.arguments(&["--genesis-block", &path.display().to_string()]);
            }
            NodeBlock0::Hash(hash) => {
                command.arguments(&["--genesis-block-hash", &hash.to_string()]);
            }
        }

        let process = Process::spawn(command).chain_err(|| ErrorKind::CannotSpawnNode)?;

        let node = Node {
            alias: alias.into(),

            dir,

            process,

            progress_bar,
            node_settings: node_settings.clone(),
            status: Arc::new(Mutex::new(Status::Running)),
        };

        node.progress_bar_start();

        Ok(node)
    }

    pub fn capture_logs(&mut self) -> impl Future<Item = (), Error = ()> {
        let stderr = self.process.stderr().take().unwrap();

        let stderr = tokio::codec::FramedRead::new(stderr, tokio::codec::LinesCodec::new());

        let progress_bar = self.progress_bar.clone();

        stderr
            .for_each(move |line| future::ok(progress_bar.log_info(&line)))
            .map_err(|err| unimplemented!("{}", err))
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
    fn new(progress_bar: ProgressBar, prefix: String) -> Self {
        ProgressBarController {
            progress_bar,
            prefix,
        }
    }

    fn log_info<M>(&self, msg: M)
    where
        M: Display,
    {
        self.log(style::info.apply_to("INFO "), msg)
    }

    fn log_err<M>(&self, msg: M)
    where
        M: Display,
    {
        self.log(style::error.apply_to("ERROR"), style::error.apply_to(msg))
    }

    fn log<L, M>(&self, lvl: L, msg: M)
    where
        L: Display,
        M: Display,
    {
        self.progress_bar.println(format!(
            "[{}][{}{}]: {}",
            lvl,
            *style::icons::jormungandr,
            style::binary.apply_to(&self.prefix),
            msg,
        ))
    }
}

impl std::ops::Deref for ProgressBarController {
    type Target = ProgressBar;
    fn deref(&self) -> &Self::Target {
        &self.progress_bar
    }
}

impl Future for Node {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.process.poll() {
            Err(err) => {
                self.progress_bar.log_err(&err);
                self.progress_bar_failure();
                self.set_status(Status::Failure);
                Err(())
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(status)) => {
                if status.success() {
                    self.progress_bar_success();
                } else {
                    self.progress_bar.log_err(&status);
                    self.progress_bar_failure()
                }
                self.set_status(Status::Exit(status));
                Ok(Async::Ready(()))
            }
        }
    }
}

impl Control for Node {
    fn command(&self) -> &bawawa::Command {
        &self.process.command()
    }

    fn id(&self) -> u32 {
        self.process.id()
    }

    fn kill(&mut self) -> bawawa::Result<()> {
        self.process.kill()
    }
}
