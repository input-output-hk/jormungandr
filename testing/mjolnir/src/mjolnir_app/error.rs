use jormungandr_integration_tests::common::jormungandr::{JormungandrError, StartupError};
use jormungandr_testing_utils::testing::block0::GetBlock0Error;
use jormungandr_testing_utils::testing::node::RestError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MjolnirError {
    #[error("cannot query rest")]
    RestError(#[from] RestError),
    #[error("cannot bootstrap node")]
    StartupError(#[from] StartupError),
    #[error("jormungandr error")]
    JormungandrError(#[from] JormungandrError),
    #[error("node client error")]
    InternalClientError,
    #[error("pace is too low ({0})")]
    PaceTooLow(u64),
    #[error("get block0 error")]
    GetBlock0Error(#[from] GetBlock0Error),
}
