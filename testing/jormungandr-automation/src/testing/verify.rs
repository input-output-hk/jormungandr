use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    VerificationFailed(String),
}

pub fn assert_equals<A: fmt::Debug + PartialEq>(
    left: &A,
    right: &A,
    info: &str,
) -> Result<(), Error> {
    if left != right {
        return Err(Error::VerificationFailed(format!(
            "{}. {:?} vs {:?}",
            info, left, right
        )));
    }
    Ok(())
}

pub fn assert(statement: bool, info: &str) -> Result<(), Error> {
    if !statement {
        return Err(Error::VerificationFailed(info.to_string()));
    }
    Ok(())
}
