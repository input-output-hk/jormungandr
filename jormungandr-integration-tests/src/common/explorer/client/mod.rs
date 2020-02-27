use super::GraphQLQuery;
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

    pub fn run(&self, query: GraphQLQuery) -> Result<reqwest::Response, GraphQLClientError> {
        println!("running query: {:?}, against: {}", query, self.base_url);
        reqwest::Client::new()
            .post(&format!("{}", self.base_url))
            .json(&query)
            .send()
            .map_err(|e| e.into())
    }
}
