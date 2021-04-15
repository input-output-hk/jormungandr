//! REST API request tooling.
mod config;
mod v0;

use crate::utils::{io::ReadYamlError, output_format};
use config::RestArgs;
use hex::FromHexError;
#[cfg(feature = "structopt")]
use structopt::StructOpt;
use thiserror::Error;

/// Send request to node REST API
#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub enum Rest {
    /// API version 0
    V0(v0::V0),
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
