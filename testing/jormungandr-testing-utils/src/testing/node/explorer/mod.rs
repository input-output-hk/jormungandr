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
    print_log: bool,
}

impl Explorer {
    pub fn new<S: Into<String>>(address: S) -> Explorer {
        Explorer {
            client: GraphQLClient::new(address),
            print_log: true,
        }
    }

    pub fn disable_logs(&mut self) {
        self.print_log = false;
    }

    pub fn get_last_block(&self) -> Result<ExplorerLastBlock, ExplorerError> {
        let query = ExplorerLastBlock::query();
        let response = self.send_request(query)?;
        ExplorerLastBlock::try_from(response).map_err(|e| e.into())
    }

    pub fn get_transaction(&self, hash: Hash) -> Result<ExplorerTransaction, ExplorerError> {
        let query = ExplorerTransaction::query_by_id(hash);
        let response = self.send_request(query)?;
        ExplorerTransaction::try_from(response).map_err(|e| e.into())
    }

    fn send_request(&self, query: GraphQLQuery) -> Result<GraphQLResponse, ExplorerError> {
        let request_response = self.client.run(query).map_err(ExplorerError::ClientError)?;

        if self.print_log {
            println!("{:?}", request_response);
        }

        let text = request_response
            .text()
            .map_err(client::GraphQLClientError::ReqwestError)?;

        if self.print_log {
            println!("{:?}", text);
        }

        serde_json::from_str(&text).map_err(ExplorerError::SerializationError)
    }
}
