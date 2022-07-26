use graphql_client::QueryBody;
use serde::Serialize;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Clone)]
pub struct GraphQlClient {
    base_url: String,
    print_out: bool,
}

#[derive(Error, Debug)]
pub enum GraphQlClientError {
    #[error("request error")]
    ReqwestError(#[from] reqwest::Error),
}

impl GraphQlClient {
    pub fn new<S: Into<String>>(base_address: S) -> GraphQlClient {
        let base_url = format!("http://{}/graphql", base_address.into());
        GraphQlClient {
            base_url,
            print_out: true,
        }
    }

    pub fn base_url(&self) -> String {
        self.base_url.to_string()
    }

    pub fn enable_print(&mut self) {
        self.print_out = true;
    }

    pub fn disable_print(&mut self) {
        self.print_out = false;
    }

    pub fn run<T: Serialize>(
        &self,
        query: QueryBody<T>,
    ) -> Result<reqwest::blocking::Response, GraphQlClientError> {
        if self.print_out {
            println!(
                "running query: {:#?}, against: {}",
                query.query, self.base_url
            );
        }
        println!("checking if explorer is up" );
        if reqwest::blocking::Client::new()
                .head(&self.base_url)
                .send()
                .is_ok()
            {
                println!("explorer is up again at {:?}",self.base_url );
            };

        reqwest::blocking::Client::new()
            .post(&self.base_url)
            .json(&query)
            .send()
            .map_err(|e| e.into())
    }
}
