#![allow(dead_code)]

use crate::wallet::WalletProxySettings;

use crate::node::Status;
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
use iapyx::ProxyClient;
use std::process::Child;
use std::sync::{Arc, Mutex};
/// send query to a running node
pub struct WalletProxyController {
    alias: NodeAlias,
    progress_bar: ProgressBarController,
    settings: WalletProxySettings,
    status: Arc<Mutex<Status>>,
    process: Child,
    client: ProxyClient
}

impl WalletProxyController {
    pub fn new(
        alias: NodeAlias,
        progress_bar: ProgressBarController,
        settings: WalletProxySettings,
        status: Arc<Mutex<Status>>,
        process: Child,
    ) -> Self {

        let address = settings.address();
        Self {
            alias,
            progress_bar,
            settings,
            status,
            process,
            client: ProxyClient::new(address)
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

    pub fn block0(&self) -> Vec<u8> {
        self.client.block0().unwrap()
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
