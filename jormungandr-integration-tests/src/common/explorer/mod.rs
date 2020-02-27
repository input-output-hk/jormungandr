use self::{
    client::GraphQLClient,
    data::{ExplorerTransaction, GraphQLQuery, GraphQLResponse},
};
use jormungandr_lib::crypto::hash::Hash;
use std::convert::TryFrom;

mod client;
mod data;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExplorerError {
    #[error("graph client error")]
    ClientError(#[from] client::GraphQLClientError),
    #[error("json serializiation error")]
    SerializationError(#[from] serde_json::Error),
}

pub struct Explorer {
    client: GraphQLClient,
}

impl Explorer {
    pub fn new<S: Into<String>>(address: S) -> Explorer {
        Explorer {
            client: GraphQLClient::new(address),
        }
    }

    pub fn get_transaction(&self, hash: Hash) -> Result<ExplorerTransaction> {
        let query = ExplorerTransaction::query_by_id(hash);
        let response = self.client.run(query);
        let response: GraphQLResponse = serde_json::from_str(&response?.text()?)?;
        ExplorerTransaction::try_from(response).map_err(|e| e.into())
    }
}
