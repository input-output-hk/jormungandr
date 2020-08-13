#![allow(dead_code)]

use crate::data::{Fund, Proposal};
use hyper::StatusCode;
use reqwest::blocking::Response;
use thiserror::Error;

pub const API_TOKEN_HEADER: &str = "API-Token";

#[derive(Debug, Clone)]
pub struct RestClientLogger {
    enabled: bool,
}

impl RestClientLogger {
    pub fn log_request(&self, request: &str) {
        if !self.is_enabled() {
            return;
        }
        println!("Request: {:#?}", request);
    }

    pub fn log_response(&self, response: &reqwest::blocking::Response) {
        if !self.is_enabled() {
            return;
        }
        println!("Response: {:#?}", response);
    }

    pub fn log_text(&self, content: &str) {
        if !self.is_enabled() {
            return;
        }
        println!("Text: {:#?}", content);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled
    }
}

#[derive(Debug, Clone)]
pub struct VitStationRestClient {
    path_builder: RestPathBuilder,
    api_token: Option<String>,
    logger: RestClientLogger,
}

impl VitStationRestClient {
    pub fn new(address: String) -> Self {
        Self {
            api_token: None,
            path_builder: RestPathBuilder::new(address),
            logger: RestClientLogger { enabled: false },
        }
    }

    pub fn disable_logs(&mut self) {
        self.logger.set_enabled(false);
    }

    pub fn enable_logs(&mut self) {
        self.logger.set_enabled(true);
    }

    pub fn health(&self) -> Result<(), RestError> {
        self.get_and_verify_status_code(&self.path_builder.health())
            .map(|_| ())
            .map_err(|_| RestError::ServerIsNotUp)
    }

    pub fn health_raw(&self) -> Result<Response, RestError> {
        self.get(&self.path_builder.health())
            .map_err(RestError::RequestError)
    }

    pub fn funds(&self) -> Result<Fund, RestError> {
        let content = self
            .get_and_verify_status_code(&self.path_builder.funds())?
            .text()?;
        self.logger.log_text(&content);
        serde_json::from_str(&content).map_err(|e| RestError::CannotDeserializeResponse {
            source: e,
            text: content.clone(),
        })
    }

    pub fn funds_raw(&self) -> Result<Response, RestError> {
        self.get(&self.path_builder.funds())
            .map_err(RestError::RequestError)
    }

    pub fn path_builder(&self) -> &RestPathBuilder {
        &self.path_builder
    }

    pub fn proposals(&self) -> Result<Vec<Proposal>, RestError> {
        let content = self
            .get_and_verify_status_code(&self.path_builder.proposals())?
            .text()?;
        self.logger.log_text(&content);
        if content.is_empty() {
            return Ok(vec![]);
        }
        serde_json::from_str(&content).map_err(|e| RestError::CannotDeserializeResponse {
            source: e,
            text: content.clone(),
        })
    }

    pub fn proposals_raw(&self) -> Result<Response, RestError> {
        self.get(&self.path_builder.proposals())
            .map_err(RestError::RequestError)
    }

    pub fn proposal(&self, id: &str) -> Result<Proposal, RestError> {
        let response = self.proposal_raw(id)?;
        self.verify_status_code(&response)?;
        let content = response.text()?;
        self.logger.log_text(&content);
        serde_json::from_str(&content).map_err(RestError::CannotDeserialize)
    }

    pub fn proposal_raw(&self, id: &str) -> Result<Response, RestError> {
        self.get(&self.path_builder().proposal(id))
            .map_err(RestError::RequestError)
    }

    pub fn fund(&self, id: &str) -> Result<Fund, RestError> {
        let response = self.fund_raw(id)?;
        self.verify_status_code(&response)?;
        let content = response.text()?;
        self.logger.log_text(&content);
        serde_json::from_str(&content).map_err(RestError::CannotDeserialize)
    }

    pub fn fund_raw(&self, id: &str) -> Result<Response, RestError> {
        self.get(&self.path_builder().fund(id))
            .map_err(RestError::RequestError)
    }

    pub fn genesis(&self) -> Result<Vec<u8>, RestError> {
        Ok(self.genesis_raw()?.bytes()?.to_vec())
    }

    pub fn genesis_raw(&self) -> Result<Response, RestError> {
        self.get(&self.path_builder.genesis())
            .map_err(RestError::RequestError)
    }

    pub fn get(&self, path: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
        self.logger.log_request(path);
        let client = reqwest::blocking::Client::new();
        let mut res = client.get(path);

        if let Some(api_token) = &self.api_token {
            res = res.header(API_TOKEN_HEADER, api_token.to_string());
        }
        let response = res.send()?;
        self.logger.log_response(&response);
        Ok(response)
    }

    fn get_and_verify_status_code(
        &self,
        path: &str,
    ) -> Result<reqwest::blocking::Response, RestError> {
        let response = self.get(path)?;
        self.verify_status_code(&response)?;
        Ok(response)
    }

    fn verify_status_code(&self, response: &Response) -> Result<(), RestError> {
        if !response.status().is_success() {
            return Err(RestError::ErrorStatusCode(response.status()));
        }
        Ok(())
    }

    pub fn disable_log(&mut self) {
        self.logger.set_enabled(false);
    }

    pub fn set_api_token(&mut self, token: String) {
        self.api_token = Some(token);
    }

    pub fn post(&self, path: &str, data: String) -> Result<serde_json::Value, RestError> {
        let client = reqwest::blocking::Client::new();
        let mut res = client.post(path).body(String::into_bytes(data));

        if let Some(api_token) = &self.api_token {
            res = res.header(API_TOKEN_HEADER, api_token.to_string());
        }
        let response = res.send()?;
        self.logger.log_response(&response);
        let result = response.text();
        Ok(serde_json::from_str(&result?)?)
    }
}

#[derive(Debug, Clone)]
pub struct RestPathBuilder {
    address: String,
    root: String,
}

impl RestPathBuilder {
    pub fn new<S: Into<String>>(address: S) -> Self {
        RestPathBuilder {
            root: "/api/v0/".to_string(),
            address: address.into(),
        }
    }

    pub fn proposals(&self) -> String {
        self.path("proposals")
    }

    pub fn funds(&self) -> String {
        self.path("fund")
    }

    pub fn proposal(&self, id: &str) -> String {
        self.path(&format!("proposals/{}", id))
    }

    pub fn fund(&self, id: &str) -> String {
        self.path(&format!("fund/{}", id))
    }

    pub fn genesis(&self) -> String {
        self.path("block0")
    }

    pub fn graphql(&self) -> String {
        self.path("graphql")
    }

    pub fn health(&self) -> String {
        self.path("health")
    }

    fn path(&self, path: &str) -> String {
        format!("http://{}{}{}", self.address, self.root, path)
    }
}

#[derive(Debug, Error)]
pub enum RestError {
    #[error("could not deserialize response {text}, due to: {source}")]
    CannotDeserializeResponse {
        source: serde_json::Error,
        text: String,
    },
    #[error("could not deserialize response")]
    CannotDeserialize(#[from] serde_json::Error),
    #[error("could not send reqeuest")]
    RequestError(#[from] reqwest::Error),
    #[error("server is not up")]
    ServerIsNotUp,
    #[error("Error code recieved: {0}")]
    ErrorStatusCode(StatusCode),
}
