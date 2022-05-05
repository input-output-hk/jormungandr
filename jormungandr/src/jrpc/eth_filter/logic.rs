use super::Error;
use crate::{
    context::Context,
    jrpc::eth_types::{filter::Filter, number::Number},
};

pub fn new_filter(_filter: Filter, _context: &Context) -> Result<Number, Error> {
    // TODO implement
    Ok(0.into())
}

pub fn new_block_filter(_context: &Context) -> Result<Number, Error> {
    // TODO implement
    Ok(0.into())
}

pub fn new_pending_transaction_filter(_context: &Context) -> Result<Number, Error> {
    // TODO implement
    Ok(0.into())
}

pub fn uninstall_filter(_filter_id: Number, _context: &Context) -> Result<bool, Error> {
    // TODO implement
    Ok(true)
}

pub fn get_filter_changes(_filter_id: Number, _context: &Context) -> Result<(), Error> {
    // TODO implement
    Ok(())
}

pub fn get_filter_logs(_filter_id: Number, _context: &Context) -> Result<(), Error> {
    // TODO implement
    Ok(())
}
