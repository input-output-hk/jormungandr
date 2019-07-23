mod v0;

use jcli_app::utils::{host_addr, io::ReadYamlError, output_format, CustomErrorFiller};
use structopt::StructOpt;

/// Send request to node REST API
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Rest {
    /// API version 0
    V0(v0::V0),
}

const DESERIALIZATION_ERROR_MSG: &'static str = "node returned malformed data";

custom_error! {pub Error
    ReqwestError { source: reqwest::Error } = @{ reqwest_error_msg(source) },
    HostAddrError { source: host_addr::Error } = "invalid host address",
    DeserializationError { source: serde_json::Error } = @{{ let _ = source; DESERIALIZATION_ERROR_MSG }},
    InputFragmentMalformed { source: std::io::Error,  filler: CustomErrorFiller}  =  "input is not a valid fragment",
    OutputFormatFailed { source: output_format::Error } = "formatting output failed",
    InputFileInvalid { source: std::io::Error } = "could not read input file",
    InputFileYamlMalformed { source: serde_yaml::Error } = "input yaml is not valid",
    InputSerializationFailed { source: serde_json::Error, filler: CustomErrorFiller } = "failed to serialize input",
    InputHexMalformed { source: hex::Error } = "input hex encoding is not valid",
}

impl From<ReadYamlError> for Error {
    fn from(error: ReadYamlError) -> Self {
        match error {
            ReadYamlError::Io { source } => Error::InputFileInvalid { source },
            ReadYamlError::Yaml { source } => Error::InputFileYamlMalformed { source },
        }
    }
}

fn reqwest_error_msg(err: &reqwest::Error) -> &'static str {
    if err.is_timeout() {
        "connection with node timed out"
    } else if err.is_http() {
        "could not connect with node"
    } else if err.is_serialization() {
        DESERIALIZATION_ERROR_MSG
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
