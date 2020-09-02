use self::{
    client::GraphQLClient,
    data::{last_block, transaction_by_id, LastBlock, TransactionById},
};
use graphql_client::GraphQLQuery;
use graphql_client::*;
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
    #[error("request error")]
    ReqwestError(#[from] reqwest::Error),
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

    pub fn get_last_block(&self) -> Result<Response<last_block::ResponseData>, ExplorerError> {
        let query = LastBlock::build_query(last_block::Variables);
        let response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        let response_body: Response<last_block::ResponseData> = response.json()?;
        self.print_log(&response_body);
        Ok(response_body)
    }

    pub fn get_transaction(
        &self,
        hash: Hash,
    ) -> Result<Response<transaction_by_id::ResponseData>, ExplorerError> {
        let query = TransactionById::build_query(transaction_by_id::Variables {
            id: hash.to_string(),
        });
        let response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        let response_body: Response<transaction_by_id::ResponseData> = response.json()?;
        self.print_log(&response_body);
        Ok(response_body)
    }

    fn print_log<T: std::fmt::Debug>(&self, response: &Response<T>) {
        if self.print_log {
            println!("Response: {:?}", &response);
        }
    }
}
