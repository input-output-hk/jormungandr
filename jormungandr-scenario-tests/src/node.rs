use crate::{scenario::settings::NodeSetting, style, Context, NodeAlias};
use bawawa::{Control, Process};
use chain_impl_mockchain::block::{Block, HeaderHash};
use indicatif::ProgressBar;
use mktemp::Temp;
use rand_core::RngCore;
use std::{
    path::PathBuf,
    process::ExitStatus,
    sync::{Arc, Mutex},
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

        InvalidHeaderHash {
            description("Invalid header hash"),
        }

        InvalidBlock {
            description("Invalid block"),
        }

        NodeStopped (status: Status) {
            description("the node is no longer running"),
        }
    }
}

pub enum NodeBlock0 {
    Hash(HeaderHash),
    File(PathBuf),
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
pub struct NodeController {
    alias: NodeAlias,
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
    temp_dir: Temp,

    process: Process,

    progress_bar: ProgressBarController,
    node_settings: NodeSetting,
    status: Arc<Mutex<Status>>,
}

const NODE_CONFIG: &str = "node_config.yaml";
const NODE_SECRET: &str = "node_secret.yaml";

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

    fn get(&self, path: &str) -> Result<reqwest::Response> {
        let node_settings = &self.settings;

        let address = node_settings.config.rest.listen.clone();

        self.progress_bar.log_info(format!("GET '{}'", path));

        match reqwest::get(&format!("http://{}/api/v0/{}", address, path)) {
            Err(err) => {
                self.progress_bar
                    .log_err(format!("Failed to send request {}", &err));
                Err(err.into())
            }
            Ok(r) => Ok(r),
        }
    }

    pub fn get_tip(&self) -> Result<HeaderHash> {
        let hash = self.get("tip")?.text()?;

        let hash = hash.parse().chain_err(|| ErrorKind::InvalidHeaderHash)?;

        self.progress_bar.log_info(format!("tip '{}'", hash));

        Ok(hash)
    }

    pub fn get_block(&self, header_hash: &HeaderHash) -> Result<Block> {
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
        NodeController {
            alias: self.alias().clone(),
            settings: self.node_settings.clone(),
            status: self.status.clone(),
            progress_bar: self.progress_bar.clone(),
        }
    }

    pub fn spawn<R: RngCore>(
        context: &Context<R>,
        progress_bar: ProgressBar,
        alias: &str,
        node_settings: &NodeSetting,
        block0: NodeBlock0,
    ) -> Result<Self> {
        let mut command = context.jormungandr().clone();
        let temp_dir = Temp::new_dir().chain_err(|| ErrorKind::CannotCreateTemporaryDirectory)?;

        let progress_bar = ProgressBarController::new(
            progress_bar,
            format!("{}@{}", alias, node_settings.config().rest.listen),
        );

        let config_file = {
            let mut dir = temp_dir.clone().release();
            dir.push(NODE_CONFIG);
            dir
        };
        let config_secret = {
            let mut dir = temp_dir.clone().release();
            dir.push(NODE_SECRET);
            dir
        };

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

            temp_dir,

            process,

            progress_bar,
            node_settings: node_settings.clone(),
            status: Arc::new(Mutex::new(Status::Running)),
        };

        node.progress_bar_start();

        Ok(node)
    }

    fn progress_bar_start(&self) {
        self.progress_bar
            .set_style(indicatif::ProgressStyle::default_spinner().tick_chars("/|\\- "));
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
