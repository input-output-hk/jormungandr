use graphql_client::QueryBody;
use serde::Serialize;
use thiserror::Error;
pub struct GraphQLClient {
    base_url: String,
}

#[derive(Error, Debug)]
pub enum GraphQLClientError {
    #[error("request error")]
    ReqwestError(#[from] reqwest::Error),
}

impl GraphQLClient {
    pub fn new<S: Into<String>>(base_address: S) -> GraphQLClient {
        let base_url = format!("http://{}/explorer/graphql", base_address.into());
        GraphQLClient { base_url }
    }

    pub fn base_url(&self) -> String {
        self.base_url.to_string()
    }

    pub fn run<T: Serialize>(
        &self,
        query: QueryBody<T>,
    ) -> Result<reqwest::blocking::Response, GraphQLClientError> {
        println!(
            "running query: {:?}, against: {}",
            query.query, self.base_url
        );

        reqwest::blocking::Client::new()
            .post(&self.base_url)
            .json(&query)
            .send()
            .map_err(|e| e.into())
    }
}
