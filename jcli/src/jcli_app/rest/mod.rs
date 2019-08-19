mod v0;

use hex::FromHexError;
use jcli_app::utils::rest_api::{self, DESERIALIZATION_ERROR_MSG};
use jcli_app::utils::{host_addr, io::ReadYamlError, output_format, CustomErrorFiller};
use structopt::StructOpt;

/// Send request to node REST API
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Rest {
    /// API version 0
    V0(v0::V0),
}

custom_error! {pub Error
    RestError { source: rest_api::Error } = "failed to make a REST request",
    HostAddrError { source: host_addr::Error } = "invalid host address",
    DeserializationError { source: serde_json::Error } = @{{ let _ = source; DESERIALIZATION_ERROR_MSG }},
    InputFragmentMalformed { source: std::io::Error,  filler: CustomErrorFiller}  =  "input is not a valid fragment",
    OutputFormatFailed { source: output_format::Error } = "formatting output failed",
    InputFileInvalid { source: std::io::Error } = "could not read input file",
    InputFileYamlMalformed { source: serde_yaml::Error } = "input yaml is not valid",
    InputSerializationFailed { source: serde_json::Error, filler: CustomErrorFiller } = "failed to serialize input",
    InputHexMalformed { source: FromHexError } = "input hex encoding is not valid",
}

impl From<ReadYamlError> for Error {
    fn from(error: ReadYamlError) -> Self {
        match error {
            ReadYamlError::Io { source } => Error::InputFileInvalid { source },
            ReadYamlError::Yaml { source } => Error::InputFileYamlMalformed { source },
        }
    }
}

impl Rest {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Rest::V0(v0) => v0.exec(),
        }
    }
}
