use hyper::StatusCode;
use thiserror::Error;

pub struct ProxyClient {
    address: String,
    debug: bool,
}

impl ProxyClient {
    pub fn new(address: String) -> Self {
        Self {
            address,
            debug: false,
        }
    }

    pub fn enable_debug(&mut self) {
        self.debug = true;
    }

    pub fn disable_debug(&mut self) {
        self.debug = false;
    }

    pub fn print_response(&self, response: &reqwest::blocking::Response) {
        if self.debug {
            println!("Response: {:?}", response);
        }
    }

    pub fn print_request_path(&self, path: &str) {
        if self.debug {
            println!("Request: {}", path);
        }
    }

    pub fn block0(&self) -> Result<Vec<u8>, Error> {
        let response = reqwest::blocking::get(&self.path("block0"))?;
        self.print_response(&response);
        Ok(response.bytes()?.to_vec())
    }

    fn path(&self, path: &str) -> String {
        let path = format!("{}/{}", self.address, path);
        self.print_request_path(&path);
        path
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
