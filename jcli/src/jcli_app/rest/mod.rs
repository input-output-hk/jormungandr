mod v0;

use crate::jcli_app::utils::{
    host_addr,
    io::ReadYamlError,
    output_format,
    rest_api::{self, DESERIALIZATION_ERROR_MSG},
};
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
    RestError(#[from] rest_api::Error),
    #[error("invalid host address")]
    HostAddrError(#[from] host_addr::Error),
    #[error("{}", DESERIALIZATION_ERROR_MSG)]
    DeserializationError(#[from] serde_json::Error),
    #[error("input is not a valid fragment")]
    InputFragmentMalformed(#[source] std::io::Error),
    #[error("formatting output failed")]
    OutputFormatFailed(#[from] output_format::Error),
    #[error("could not read input file")]
    InputFileInvalid(#[from] std::io::Error),
    #[error("input yaml is not valid")]
    InputFileYamlMalformed(#[from] serde_yaml::Error),
    #[error("input hex encoding is not valid")]
    InputHexMalformed(#[from] FromHexError),
}

impl From<ReadYamlError> for Error {
    fn from(error: ReadYamlError) -> Self {
        match error {
            ReadYamlError::Io(source) => Error::InputFileInvalid(source),
            ReadYamlError::Yaml(source) => Error::InputFileYamlMalformed(source),
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
