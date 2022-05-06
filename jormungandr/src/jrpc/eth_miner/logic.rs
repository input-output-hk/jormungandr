use crate::{context::Context, jrpc::eth_types::number::Number};

use super::Error;

pub fn mining(_context: &Context) -> Result<bool, Error> {
    // TODO implement
    Ok(true.into())
}

pub fn hashrate(_context: &Context) -> Result<Number, Error> {
    // TODO implement
    Ok(0.into())
}

pub fn get_work(_context: &Context) -> Result<(), Error> {
    // TODO implement
    Ok(())
}

pub fn submit_work(_context: &Context) -> Result<(), Error> {
    // TODO implement
    Ok(())
}

pub fn submit_hashrate(_context: &Context) -> Result<(), Error> {
    // TODO implement
    Ok(())
}
