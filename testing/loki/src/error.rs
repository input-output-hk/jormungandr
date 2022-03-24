use chain_core::property::ReadError;
use jormungandr_lib::interfaces::Block0ConfigurationError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Read error: {0}")]
    Read(#[from] ReadError),
    #[error("Could not parse YAML file: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Block0 error: {0}")]
    Block0(#[from] Block0ConfigurationError),
}
