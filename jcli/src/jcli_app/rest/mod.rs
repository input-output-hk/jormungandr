mod v0;

use crate::jcli_app::utils::rest_api::{self, DESERIALIZATION_ERROR_MSG};
use crate::jcli_app::utils::{host_addr, io::ReadYamlError, output_format};
use hex::FromHexError;
use structopt::StructOpt;
use thiserror::Error;

/// Send request to node REST API
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Rest {
    /// API version 0
    V0(v0::V0),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to make a REST request")]
    RestError {
        #[from]
        source: rest_api::Error,
    },
    #[error("invalid host address")]
    HostAddrError {
        #[from]
        source: host_addr::Error,
    },
    #[error("{}", DESERIALIZATION_ERROR_MSG)]
    DeserializationError {
        #[from]
        source: serde_json::Error,
    },
    #[error("input is not a valid fragment")]
    InputFragmentMalformed {
        #[source]
        source: std::io::Error,
    },
    #[error("formatting output failed")]
    OutputFormatFailed {
        #[from]
        source: output_format::Error,
    },
    #[error("could not read input file")]
    InputFileInvalid {
        #[from]
        source: std::io::Error,
    },
    #[error("input yaml is not valid")]
    InputFileYamlMalformed {
        #[from]
        source: serde_yaml::Error,
    },
    #[error("failed to serialize input")]
    InputSerializationFailed {
        #[source]
        source: serde_json::Error,
    },
    #[error("input hex encoding is not valid")]
    InputHexMalformed {
        #[from]
        source: FromHexError,
    },
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
