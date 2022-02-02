use jormungandr_automation::jormungandr::{JormungandrError, RestError, StartupError};
use jormungandr_automation::testing::block0::GetBlock0Error;
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
