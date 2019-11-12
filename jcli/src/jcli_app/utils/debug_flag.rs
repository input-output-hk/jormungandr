use crate::jcli_app::utils::rest_api::{RestApiRequestBody, RestApiResponse};
use reqwest::Request;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct DebugFlag {
    /// print additional debug information to stderr.
    /// The output format is intentionally undocumented and unstable
    #[structopt(long)]
    debug: bool,
}

impl DebugFlag {
    pub fn write_request(&self, request: &Request, body: &RestApiRequestBody) {
        if !self.debug {
            return;
        }
        eprintln!("{:#?}", request);
        if body.has_body() {
            eprintln!("Request body:\n{}", body)
        }
    }

    pub fn write_response(&self, response: &RestApiResponse) {
        if !self.debug {
            return;
        }
        eprintln!("{:#?}", response.response());
        let body = response.body();
        if !body.is_empty() {
            eprintln!("Response body:\n{}", body)
        }
    }
}
