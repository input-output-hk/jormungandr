use hyper::StatusCode;
use thiserror::Error;

pub struct ProxyClient {
    address: String,
}

impl ProxyClient {
    pub fn new(address: String) -> Self {
        Self { address }
    }

    pub fn block0(&self) -> Result<Vec<u8>, Error> {
        Ok(reqwest::blocking::get(&self.path("block0"))?
            .bytes()?
            .to_vec())
    }

    fn path(&self, path: &str) -> String {
        format!("{}/{}", self.address, path)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not deserialize response {text}, due to: {source}")]
    CannotDeserializeResponse {
        source: serde_json::Error,
        text: String,
    },
    #[error("could not send reqeuest")]
    RequestError(#[from] reqwest::Error),
    #[error("server is not up")]
    ServerIsNotUp,
    #[error("Error code recieved: {0}")]
    ErrorStatusCode(StatusCode),
}
