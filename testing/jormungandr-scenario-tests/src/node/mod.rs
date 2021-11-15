#![allow(dead_code)]
mod legacy;

pub use legacy::LegacyNode;

use crate::{scenario::ProgressBarMode, style};
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_lib::{crypto::hash::Hash, multiaddr};
use jormungandr_testing_utils::testing::jormungandr::ShutdownError;
use jormungandr_testing_utils::testing::jormungandr::{
    JormungandrProcess, StartupError, StartupVerificationMode, Status,
};
use jormungandr_testing_utils::testing::node::Explorer;
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

use indicatif::ProgressBar;
use std::net::SocketAddr;

use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::process::ExitStatus;
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
    #[error("node '{alias}' failed to start: {e}")]
    NodeFailedToBootstrap {
        alias: String,
        #[source]
        e: StartupError,
    },
    #[error("node '{alias}' failed to shutdown, due to: {message}")]
    ShutdownProcedure {
        alias: String,
        message: String,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("node '{alias}' failed to shutdown: {e}")]
    NodeFailedToShutdown {
        alias: String,
        #[source]
        e: ShutdownError,
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
            FragmentNotInMemPoolLogs { logs, .. } | FragmentIsPendingForTooLong { logs, .. } => {
                Some(logs)
            }
            _ => None,
        };
        maybe_logs
            .into_iter()
            .map(|logs| logs.iter())
            .flatten()
            .map(String::as_str)
    }
}

#[derive(Clone)]
pub struct ProgressBarController {
    progress_bar: ProgressBar,
    prefix: String,
    logging_mode: ProgressBarMode,
}

/// Node is going to be used by the `Controller` to monitor the node process
pub struct Node {
    process: JormungandrProcess,
    progress_bar: ProgressBarController,
}

impl Node {
    pub fn new(process: JormungandrProcess, progress_bar: ProgressBarController) -> Self {
        let node = Self {
            process,
            progress_bar,
        };
        node.progress_bar_start();
        node
    }

    pub fn alias(&self) -> NodeAlias {
        self.process.alias()
    }

    pub fn controller(self) -> JormungandrProcess {
        self.process
    }

    pub fn status(&self) -> Status {
        self.process.status(&StartupVerificationMode::Rest)
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
        self.process
            .wait_for_bootstrap(&StartupVerificationMode::Rest, Duration::from_secs(150))
            .map_err(|e| Error::NodeFailedToBootstrap {
                alias: self.alias(),
                e,
            })
            .map(|_| self.progress_bar.log_info("bootstapped successfully."))
    }

    pub fn wait_for_shutdown(&mut self) -> Result<Option<ExitStatus>> {
        self.process
            .wait_for_shutdown(Duration::from_secs(150))
            .map_err(|e| {
                self.progress_bar
                    .log_info(format!("shutdown error: {}", e.to_string()));
                Error::NodeFailedToShutdown {
                    alias: self.alias(),
                    e,
                }
            })
            .map(|exit_status| {
                self.progress_bar.log_info("shutdown successfully.");
                exit_status
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
        matches!(self.status(), Status::Running)
    }

    pub fn shutdown(&mut self) -> Result<Option<ExitStatus>> {
        let message = self.rest().shutdown()?;

        if message.is_empty() {
            self.progress_bar.log_info("shuting down...");
            let exit_status = self.wait_for_shutdown();
            self.progress_bar.finish_with_message("shutdown");
            exit_status
        } else {
            Err(Error::ShutdownProcedure {
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
        self.log(style::info.apply_to("INFO"), msg)
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
                self.progress_bar.set_message(&format!(
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

    pub fn finish_with_message<M>(&self, msg: M)
    where
        M: Display,
    {
        let lvl = "INFO";

        match self.logging_mode {
            ProgressBarMode::Standard => {
                println!("[{}][{}]: {}", lvl, &self.prefix, msg);
            }
            ProgressBarMode::Monitor => {
                self.progress_bar.finish_with_message(&format!(
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
