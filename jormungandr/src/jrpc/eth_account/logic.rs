use super::Error;
use crate::context::Context;

pub fn get_transaction_count(_context: &Context) -> Result<(), Error> {
    Ok(())
}

pub fn get_balance(_context: &Context) -> Result<(), Error> {
    Ok(())
}

pub fn get_code(_context: &Context) -> Result<(), Error> {
    Ok(())
}

pub fn get_storage_at(_context: &Context) -> Result<(), Error> {
    Ok(())
}
