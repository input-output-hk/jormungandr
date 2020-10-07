#![allow(dead_code)]

use crate::wallet::WalletProxySettings;
use crate::VitStationController;
use crate::WalletProxy;
use chain_impl_mockchain::{
    block::Block,
    fragment::{Fragment, FragmentId},
    header::HeaderId,
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{
        EnclaveLeaderId, FragmentLog, LeadershipLog, Log, LogEntry, LogOutput, NodeState,
        NodeStatsDto, PeerRecord, PeerStats,
    },
};
use std::net::SocketAddr;
use vit_servicing_station_tests::common::clients::RestClient;
use vit_servicing_station_tests::common::startup::db::DbBuilder;
use vit_servicing_station_tests::common::startup::server::BootstrapCommandBuilder;

use crate::node::Status;
use assert_fs::TempDir;
use chain_impl_mockchain::certificate::VotePlan;
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
use futures::executor::block_on;
use indicatif::ProgressBar;
use rand_core::RngCore;
use vit_servicing_station_lib::db::models::vote_options::VoteOptions;
use vit_servicing_station_lib::server::settings::dump_settings_to_file;

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// send query to a running node
pub struct WalletProxyController {
    alias: NodeAlias,
    progress_bar: ProgressBarController,
    settings: WalletProxySettings,
    status: Arc<Mutex<Status>>,
    process: Child,
}

impl WalletProxyController {
    pub fn new(
        alias: NodeAlias,
        progress_bar: ProgressBarController,
        settings: WalletProxySettings,
        status: Arc<Mutex<Status>>,
        process: Child,
    ) -> Self {
        Self {
            alias,
            progress_bar,
            settings,
            status,
            process,
        }
    }

    pub fn alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn status(&self) -> Status {
        *self.status.lock().unwrap()
    }

    pub fn check_running(&self) -> bool {
        self.status() == Status::Running
    }

    pub fn address(&self) -> String {
        self.settings.address()
    }

    pub fn as_named_process(&self) -> NamedProcess {
        NamedProcess::new(self.alias().to_string(), self.process.id() as usize)
    }

    pub fn progress_bar(&self) -> &ProgressBarController {
        &self.progress_bar
    }

    pub fn settings(&self) -> &WalletProxySettings {
        &self.settings
    }

    pub fn shutdown(mut self) {
        let _ = self.process.kill();
    }
}
