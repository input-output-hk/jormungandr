use crate::jormungandr::get_available_port;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct ExplorerConfigurationBuilder {
    config: ExplorerConfiguration,
}

impl ExplorerConfigurationBuilder {
    pub fn address(mut self, address: String) -> Self {
        self.config.node_address = address;
        self
    }
    pub fn log_dir(mut self, p: Option<PathBuf>) -> Self {
        self.config.logs_dir = p;
        self
    }

    pub fn params(mut self, params: ExplorerParams) -> Self {
        self.config.params = params;
        self
    }

    pub fn build(self) -> ExplorerConfiguration {
        self.config
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExplorerConfiguration {
    pub explorer_port: u16,
    pub explorer_listen_address: String,
    pub node_address: String,
    pub logs_dir: Option<PathBuf>,
    pub storage_dir: Option<PathBuf>,
    #[serde(flatten)]
    pub params: ExplorerParams,
}

impl ExplorerConfiguration {
    pub(crate) fn explorer_listen_http_address(&self) -> String {
        format!(
            "http://{}:{}/",
            &self.explorer_listen_address, &self.explorer_port
        )
    }
}

impl Default for ExplorerConfiguration {
    fn default() -> Self {
        let explorer_port = get_available_port();
        let explorer_listen_address = "127.0.0.1".to_string();

        Self {
            explorer_port,
            explorer_listen_address,
            node_address: "127.0.0.1:8080".to_string(),
            logs_dir: Default::default(),
            storage_dir: Default::default(),
            params: Default::default(),
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ExplorerParams {
    pub query_complexity_limit: Option<u64>,
    pub query_depth_limit: Option<u64>,
    pub address_bech32_prefix: Option<String>,
}

impl ExplorerParams {
    pub fn new(
        query_complexity_limit: impl Into<Option<u64>>,
        query_depth_limit: impl Into<Option<u64>>,
        address_bech32_prefix: impl Into<Option<String>>,
    ) -> ExplorerParams {
        ExplorerParams {
            query_complexity_limit: query_complexity_limit.into(),
            query_depth_limit: query_depth_limit.into(),
            address_bech32_prefix: address_bech32_prefix.into(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for ExplorerParams {
    //Passing None we use the default values of the explorer
    //DEFAULT_QUERY_DEPTH_LIMIT= 15
    //DEFAULT_QUERY_COMPLEXITY_LIMIT= 40
    //address_bech32_prefix= addr
    fn default() -> Self {
        ExplorerParams {
            query_complexity_limit: None,
            query_depth_limit: None,
            address_bech32_prefix: None,
        }
    }
}
