use self::{
    client::GraphQLClient,
    data::{ExplorerTransaction, GraphQLQuery, GraphQLResponse},
};
use jormungandr_lib::crypto::hash::Hash;
use std::convert::TryFrom;

mod client;
mod data;

error_chain! {
    links {
        GraphQLClientError(self::client::Error, self::client::ErrorKind);
    }

    foreign_links {
        JsonError(serde_json::error::Error);
        Reqwest(reqwest::Error);
    }
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
