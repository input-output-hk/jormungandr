mod config;
pub mod v0;
pub mod v1;

use crate::jcli_lib::utils::{io::ReadYamlError, output_format};
pub use config::RestArgs;
use hex::FromHexError;
use structopt::StructOpt;
use thiserror::Error;

/// Send request to node REST API
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Rest {
    /// API version 0
    V0(v0::V0),
    /// API version 1
    V1(v1::V1),
}

#[derive(Debug, Error)]
pub enum Error {
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
    #[error("error when trying to perform an HTTP request")]
    RequestError(#[from] config::Error),
    #[error("error loading data from response")]
    SerdeError(#[from] serde_json::Error),
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
            Rest::V1(v1) => v1.exec(),
        }
    }
}
