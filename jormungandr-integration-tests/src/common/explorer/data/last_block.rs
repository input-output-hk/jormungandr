use super::{GraphQLQuery, GraphQLResponse};
use chain_impl_mockchain::block::BlockDate;
use jormungandr_lib::{crypto::hash::Hash, interfaces::BlockDate as BlockDateLib};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExplorerLastBlock {
    id: Hash,
    chain_length: u32,
    date: BlockDateLib,
}

impl ExplorerLastBlock {
    pub fn id(&self) -> Hash {
        self.id
    }
}

impl TryFrom<GraphQLResponse> for ExplorerLastBlock {
    type Error = serde_json::Error;

    fn try_from(response: GraphQLResponse) -> Result<Self, Self::Error> {
        let epoch_str: String = serde_json::from_str(
            &response.data["status"]["latestBlock"]["date"]["epoch"]["id"].to_string(),
        )?;
        let slot_id_str: String = serde_json::from_str(
            &response.data["status"]["latestBlock"]["date"]["slot"].to_string(),
        )?;
        let chain_length_str: String = serde_json::from_str(
            &response.data["status"]["latestBlock"]["chainLength"].to_string(),
        )?;

        let block_date = BlockDate {
            epoch: epoch_str.parse().unwrap(),
            slot_id: slot_id_str.parse().unwrap(),
        };

        Ok(ExplorerLastBlock {
            id: serde_json::from_str(&response.data["status"]["latestBlock"]["id"].to_string())?,
            chain_length: chain_length_str.parse().unwrap(),
            date: block_date.into(),
        })
    }
}

impl ExplorerLastBlock {
    pub fn query() -> GraphQLQuery {
        GraphQLQuery {
            query: r#" query {	status { latestBlock {  id,  chainLength,  date {  epoch {  id  },  slot  } }  }}"#
            .to_owned(),
        }
    }
}
