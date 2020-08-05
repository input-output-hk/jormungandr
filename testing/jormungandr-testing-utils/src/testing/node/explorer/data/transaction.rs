use super::{GraphQLQuery, GraphQLResponse};
use jormungandr_lib::crypto::hash::Hash;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExplorerTransaction {
    pub id: Hash,
}

impl TryFrom<GraphQLResponse> for ExplorerTransaction {
    type Error = serde_json::Error;

    fn try_from(response: GraphQLResponse) -> Result<ExplorerTransaction, Self::Error> {
        Ok(ExplorerTransaction {
            id: serde_json::from_str(&response.data["transaction"]["id"].to_string())?,
        })
    }
}

impl ExplorerTransaction {
    pub fn query_by_id(hash: Hash) -> GraphQLQuery {
        GraphQLQuery {
            query: format!(r#"{{ transaction(id: "{}") {{ id }} }}"#, hash.into_hash()),
        }
    }
}
