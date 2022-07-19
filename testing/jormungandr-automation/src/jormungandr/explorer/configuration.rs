use crate::testing::configuration::get_explorer_app;
#[derive(Clone)]
pub struct ExplorerParams {
    pub query_complexity_limit: Option<String>,
    pub query_depth_limit: Option<String>,
    pub address_bech32_prefix: Option<String>,
}

impl ExplorerParams {
    pub fn new(
        query_complexity_limit: impl Into<Option<String>>,
        query_depth_limit: impl Into<Option<String>>,
        address_bech32_prefix: impl Into<Option<String>>,
    ) -> ExplorerParams {
        ExplorerParams {
            query_complexity_limit: query_complexity_limit.into(),
            query_depth_limit: query_depth_limit.into(),
            address_bech32_prefix: address_bech32_prefix.into(),
        }
    }
}

impl Default for ExplorerParams {
    fn default() -> Self {
        ExplorerParams {
            query_complexity_limit: None,
            query_depth_limit: None,
            address_bech32_prefix: None,
        }
    }
}
