mod last_block;
mod transaction;
pub use last_block::ExplorerLastBlock;
pub use transaction::ExplorerTransaction;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphQLResponse {
    data: serde_json::Value,
    errors: Option<serde_json::Value>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphQLQuery {
    query: String,
}
