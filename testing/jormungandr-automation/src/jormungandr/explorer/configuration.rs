#[derive(Clone)]
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
    fn default() -> Self {
        ExplorerParams {
            query_complexity_limit: None,
            query_depth_limit: None,
            address_bech32_prefix: None,
        }
    }
}
