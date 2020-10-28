mod controller;
mod settings;

use chain_impl_mockchain::fragment::FragmentId;

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

use crate::node::ProgressBarController;
use indicatif::ProgressBar;
use rand_core::RngCore;

use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{node::Status, style, Context};

pub use controller::WalletProxyController;
pub use jormungandr_testing_utils::testing::network_builder::WalletProxySettings;

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
    #[error("port already binded: {0}")]
    PortAlreadyBinded(u16),
    #[error("no wallet proxy defined in settings")]
    NoWalletProxiesDefinedInSettings,
    #[error("fragment logs in an invalid format")]
    InvalidFragmentLogs(#[source] serde_json::Error),
    #[error("node stats in an invalid format")]
    InvalidNodeStats(#[source] RestError),
    #[error("network stats in an invalid format")]
    InvalidNetworkStats(#[source] serde_json::Error),
    #[error("leaders ids in an invalid format")]
    InvalidEnclaveLeaderIds(#[source] serde_json::Error),
    #[error("node '{alias}' failed to start after {} s", .duration.as_secs())]
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

/// Node is going to be used by the `Controller` to monitor the node process
///
/// To send queries to the Node, use the `NodeController`
pub struct WalletProxy {
    alias: NodeAlias,
    process: Child,
    status: Arc<Mutex<Status>>,
    progress_bar: ProgressBarController,
    settings: WalletProxySettings,
}

impl WalletProxy {
    pub fn alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn address(&self) -> String {
        self.settings.address()
    }

    pub fn controller(self) -> WalletProxyController {
        WalletProxyController::new(
            self.alias().clone(),
            self.progress_bar.clone(),
            self.settings.clone(),
            self.status.clone(),
            self.process,
        )
    }

    pub fn spawn<R: RngCore>(
        context: &Context<R>,
        progress_bar: ProgressBar,
        alias: &str,
        mut settings: WalletProxySettings,
        node_setting: &NodeSetting,
        block0: &Path,
        working_dir: &Path,
    ) -> Result<Self> {
        let dir = working_dir.join(alias);
        std::fs::DirBuilder::new().recursive(true).create(&dir)?;

        let progress_bar = ProgressBarController::new(
            progress_bar,
            format!("{}@{}", alias, settings.address()),
            context.progress_bar_mode(),
        );

        settings.node_backend_address = Some(node_setting.config().rest.listen.clone());

        let mut command = Command::new("iapyx-proxy");
        command
            .arg("--address")
            .arg(settings.base_address().to_string())
            .arg("--vit-address")
            .arg(&settings.base_vit_address().to_string())
            .arg("--node-address")
            .arg(&settings.base_node_backend_address().unwrap().to_string())
            .arg("--block0")
            .arg(block0.to_str().unwrap());

        let wallet_proxy = WalletProxy {
            alias: alias.clone().into(),
            process: command.spawn().unwrap(),
            progress_bar,
            settings,
            status: Arc::new(Mutex::new(Status::Running)),
        };

        wallet_proxy.progress_bar_start();
        wallet_proxy
            .progress_bar
            .log_info(&format!("{} bootstrapping: {:?}", alias, command));
        Ok(wallet_proxy)
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
            self.address(),
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
