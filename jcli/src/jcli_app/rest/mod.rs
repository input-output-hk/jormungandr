mod v0;

use jcli_app::utils::{host_addr, output_format};
use structopt::StructOpt;

/// Send request to node REST API
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Rest {
    /// API version 0
    V0(v0::V0),
}

const SERIALIZATION_ERROR_MSG: &'static str = "node returned malformed data";

custom_error! {pub Error
    ReqwestError { source: reqwest::Error } = @{ reqwest_error_msg(source) },
    HostAddrError { source: host_addr::Error } = "invalid host address",
    SerializationError { source: serde_json::Error } = @{{ let _ = source; SERIALIZATION_ERROR_MSG }},
    OutputFormatFailed { source: output_format::Error } = "formatting output failed",
    InputFileInvalid { source: std::io::Error } = "could not read input file",
    InputHexMalformed { source: hex::Error } = "input hex encoding is not valid",
}

fn reqwest_error_msg(err: &reqwest::Error) -> &'static str {
    if err.is_timeout() {
        "connection with node timed out"
    } else if err.is_http() {
        "could not connect with node"
    } else if err.is_serialization() {
        SERIALIZATION_ERROR_MSG
    } else if err.is_redirect() {
        "redirecting error while connecting with node"
    } else if err.is_client_error() {
        "node rejected request because of invalid parameters"
    } else if err.is_server_error() {
        "node internal error"
    } else {
        "communication with node failed in unexpected way"
    }
}

impl Rest {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Rest::V0(v0) => v0.exec(),
        }
    }
}
