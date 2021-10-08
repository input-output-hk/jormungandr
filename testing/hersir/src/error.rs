use jormungandr_testing_utils::testing::network::controller::ControllerError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_yaml::Error),
    #[error("Circular dependency in network topology")]
    CircularTrust,
    #[error("Controller error: {0}")]
    Controller(#[from] ControllerError),
    #[error("INTERNAL ERROR: {0}")]
    Internal(String),
}
