use self::{
    client::GraphQLClient,
    data::{ExplorerLastBlock, ExplorerTransaction, GraphQLQuery, GraphQLResponse},
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

    pub fn get_last_block(&self) -> Result<ExplorerLastBlock, ExplorerError> {
        let query = ExplorerLastBlock::query();
        let request_response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        println!("{:?}", request_response);
        let text = request_response
            .text()
            .map_err(client::GraphQLClientError::ReqwestError)?;
        println!("{:?}", text);
        let response: GraphQLResponse =
            serde_json::from_str(&text).map_err(ExplorerError::SerializationError)?;
        ExplorerLastBlock::try_from(response).map_err(|e| e.into())
    }

    pub fn get_transaction(&self, hash: Hash) -> Result<ExplorerTransaction, ExplorerError> {
        let query = ExplorerTransaction::query_by_id(hash);
        let request_response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        println!("{:?}", request_response);
        let text = request_response
            .text()
            .map_err(client::GraphQLClientError::ReqwestError)?;
        println!("{:?}", text);
        let response: GraphQLResponse =
            serde_json::from_str(&text).map_err(ExplorerError::SerializationError)?;
        ExplorerTransaction::try_from(response).map_err(|e| e.into())
    }
}
