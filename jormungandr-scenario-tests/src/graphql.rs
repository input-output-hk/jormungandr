/* Helpers to make GraphQL queries easier, probably could be replaced by some graphl library */

use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_lib::crypto::hash::Hash;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphQLResponse {
    data: serde_json::Value,
    errors: Option<serde_json::Value>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphQLQuery {
    query: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExplorerTransaction {
    pub id: Hash,
}

impl TryFrom<GraphQLResponse> for ExplorerTransaction {
    type Error = serde_json::Error;

    fn try_from(response: GraphQLResponse) -> Result<ExplorerTransaction, Self::Error> {
        // FIXME: Do this properly
        Ok(ExplorerTransaction {
            id: serde_json::from_str(&response.data["transaction"]["id"].to_string())?,
        })
    }
}

impl ExplorerTransaction {
    pub fn build_query(id: FragmentId) -> GraphQLQuery {
        let hash = Hash::from(id);
        GraphQLQuery {
            query: format!(r#"{{transaction(id: "{}") {{ id }} }}"#, hash),
        }
    }
}
