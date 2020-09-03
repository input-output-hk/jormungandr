use self::{
    client::GraphQLClient,
    data::{
        address, all_blocks, all_stake_pools, all_vote_plans, block_by_chain_length, epoch,
        last_block, stake_pool, status, transaction_by_id, Address, AllBlocks, AllStakePools,
        AllVotePlans, BlockByChainLength, Epoch, LastBlock, StakePool, Status, TransactionById,
    },
};
use graphql_client::GraphQLQuery;
use graphql_client::*;
use jormungandr_lib::crypto::hash::Hash;
use std::str::FromStr;
mod client;
mod data;
use data::PoolId;
use jortestkit::file;
use std::path::Path;
use std::path::PathBuf;
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

    pub fn uri(&self) -> String {
        self.client.base_url()
    }

    pub fn disable_logs(&mut self) {
        self.print_log = false;
    }

    pub fn address<S: Into<String>>(
        &self,
        bech32_address: S,
    ) -> Result<Response<address::ResponseData>, ExplorerError> {
        let query = Address::build_query(address::Variables {
            bech32: bech32_address.into(),
        });
        let response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        let response_body: Response<address::ResponseData> = response.json()?;
        self.print_log(&response_body);
        Ok(response_body)
    }

    pub fn stake_pools(
        &self,
        limit: i64,
    ) -> Result<Response<all_stake_pools::ResponseData>, ExplorerError> {
        let query = AllStakePools::build_query(all_stake_pools::Variables { first: limit });
        let response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        let response_body = response.json()?;
        self.print_log(&response_body);
        Ok(response_body)
    }

    pub fn blocks(&self, limit: i64) -> Result<Response<all_blocks::ResponseData>, ExplorerError> {
        let query = AllBlocks::build_query(all_blocks::Variables {
            first: 0,
            last: limit,
        });
        let response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        //   println!("{:?}",response.text()?);
        let response_body = response.json()?;
        self.print_log(&response_body);

        Ok(response_body)
        //   panic!()
    }

    pub fn last_block(&self) -> Result<Response<last_block::ResponseData>, ExplorerError> {
        let query = LastBlock::build_query(last_block::Variables);
        let response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        let response_body = response.json()?;
        self.print_log(&response_body);
        Ok(response_body)
    }

    pub fn block_at_chain_length(
        &self,
        length: u32,
    ) -> Result<Response<block_by_chain_length::ResponseData>, ExplorerError> {
        let query = BlockByChainLength::build_query(block_by_chain_length::Variables {
            length: length.to_string(),
        });
        let response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        let response_body = response.json()?;
        self.print_log(&response_body);
        Ok(response_body)
    }

    pub fn epoch(
        &self,
        epoch_number: u32,
        limit: i64,
    ) -> Result<Response<epoch::ResponseData>, ExplorerError> {
        let query = Epoch::build_query(epoch::Variables {
            id: epoch_number.to_string(),
            blocks_limit: limit,
        });
        let response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        let response_body = response.json()?;
        self.print_log(&response_body);
        Ok(response_body)
    }

    pub fn stake_pool(
        &self,
        id: PoolId,
        limit: i64,
    ) -> Result<Response<stake_pool::ResponseData>, ExplorerError> {
        let query = StakePool::build_query(stake_pool::Variables { id, first: limit });
        let response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        let response_body = response.json()?;
        self.print_log(&response_body);
        Ok(response_body)
    }

    pub fn status(&self) -> Result<Response<status::ResponseData>, ExplorerError> {
        let query = Status::build_query(status::Variables);
        let response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        let response_body = response.json()?;
        self.print_log(&response_body);
        Ok(response_body)
    }

    pub fn vote_plans(
        &self,
        limit: i64,
    ) -> Result<Response<all_vote_plans::ResponseData>, ExplorerError> {
        let query = AllVotePlans::build_query(all_vote_plans::Variables { first: limit });
        let response = self.client.run(query).map_err(ExplorerError::ClientError)?;
        let response_body = response.json()?;
        self.print_log(&response_body);
        Ok(response_body)
    }

    pub fn transaction(
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

pub fn compare_schema<P: AsRef<Path>>(actual_schema_path: P) {
    let expected_schema_path =
        PathBuf::from_str("./jormungandr-testing-utils/resources/explorer/graphql/schema.graphql")
            .unwrap();

    if !file::have_the_same_content(actual_schema_path.as_ref(), &expected_schema_path) {
        file::copy_file(actual_schema_path.as_ref(), &expected_schema_path, true);
        println!("discrepancies detected, already replaced file with new content. Please commit to update schema");
    }
}
