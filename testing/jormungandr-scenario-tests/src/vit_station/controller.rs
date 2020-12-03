#![allow(dead_code)]

use crate::{style, Context};

use super::DbGenerator;
use std::net::SocketAddr;
use vit_servicing_station_lib::db::models::proposals::Proposal;
use vit_servicing_station_tests::common::clients::RestClient;
use vit_servicing_station_tests::common::startup::server::BootstrapCommandBuilder;

use crate::node::Status;
use chain_impl_mockchain::testing::scenario::template::VotePlanDef;
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
pub type VitStationSettings = vit_servicing_station_lib::server::settings::ServiceSettings;
use indicatif::ProgressBar;
use rand_core::RngCore;
use vit_servicing_station_lib::server::settings::dump_settings_to_file;

use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Child;
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
    RestError(#[from] vit_servicing_station_tests::common::clients::RestError),
    #[error("cannot spawn the node")]
    CannotSpawnNode(#[source] io::Error),
    #[error("port already binded: {0}")]
    PortAlreadyBinded(u16),
    #[error("no vit station defined in settings")]
    NoVitStationDefinedInSettings,
    #[error("fragment logs in an invalid format")]
    InvalidFragmentLogs(#[source] serde_json::Error),
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
}

/// send query to a running node
pub struct VitStationController {
    alias: NodeAlias,
    rest_client: RestClient,
    progress_bar: ProgressBarController,
    settings: VitStationSettings,
    status: Arc<Mutex<Status>>,
    process: Child,
}

/// Node is going to be used by the `Controller` to monitor the node process
///
/// To send queries to the Node, use the `NodeController`
pub struct VitStation {
    alias: NodeAlias,
    process: Child,
    status: Arc<Mutex<Status>>,
    progress_bar: ProgressBarController,
    settings: VitStationSettings,
}

const VIT_CONFIG: &str = "vit_config.yaml";
const STORAGE: &str = "storage.db";
const VIT_STATION_LOG: &str = "vit_station.log";

impl VitStationController {
    pub fn alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn status(&self) -> Status {
        *self.status.lock().unwrap()
    }

    pub fn check_running(&self) -> bool {
        self.rest_client.health().is_ok()
    }

    fn path(&self, path: &str) -> String {
        format!("{}/{}", self.base_url(), path)
    }

    pub fn address(&self) -> SocketAddr {
        self.settings.address
    }

    pub fn proposals(&self) -> Result<Vec<Proposal>> {
        Ok(self.rest_client.proposals()?)
    }

    pub fn as_named_process(&self) -> NamedProcess {
        NamedProcess::new(self.alias().to_string(), self.process.id() as usize)
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
        format!("http://{}/api/v0", self.address())
    }

    pub fn progress_bar(&self) -> &ProgressBarController {
        &self.progress_bar
    }

    pub fn shutdown(mut self) {
        let _ = self.process.kill();
    }
}

impl VitStation {
    pub fn alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn address(&self) -> SocketAddr {
        self.settings.address
    }

    pub fn controller(self) -> VitStationController {
        let rest_uri = uri_from_socket_addr(self.settings.address);

        VitStationController {
            alias: self.alias().clone(),
            rest_client: RestClient::new(rest_uri),
            process: self.process,
            status: self.status.clone(),
            progress_bar: self.progress_bar.clone(),
            settings: self.settings.clone(),
        }
    }

    pub fn spawn<R: RngCore>(
        context: &Context<R>,
        progress_bar: ProgressBar,
        alias: &str,
        settings: VitStationSettings,
        vote_plans: Vec<VotePlanDef>,
        block0: &Path,
        working_dir: &Path,
    ) -> Result<Self> {
        let dir = working_dir.join(alias);
        std::fs::DirBuilder::new().recursive(true).create(&dir)?;

        let progress_bar = ProgressBarController::new(
            progress_bar,
            format!("{}@{}", alias, settings.address.clone()),
            context.progress_bar_mode(),
        );

        let config_file = dir.join(VIT_CONFIG);
        let db_file = dir.join(STORAGE);
        dump_settings_to_file(&config_file.to_str().unwrap(), &settings).unwrap();

        DbGenerator::new(vote_plans).build(&db_file);

        let mut command_builder =
            BootstrapCommandBuilder::new(PathBuf::from("vit-servicing-station-server"));
        let mut command = command_builder
            .in_settings_file(&config_file)
            .db_url(db_file.to_str().unwrap())
            .block0_path(block0.to_str().unwrap())
            .build();

        let vit_station = VitStation {
            alias: alias.into(),
            process: command.spawn().unwrap(),
            progress_bar,
            settings,
            status: Arc::new(Mutex::new(Status::Running)),
        };

        vit_station.progress_bar_start();
        vit_station
            .progress_bar
            .log_info(&format!("{} bootstrapping: {:?}", alias, command));
        Ok(vit_station)
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
