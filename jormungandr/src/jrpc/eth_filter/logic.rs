use super::filters::FilterType;
use crate::{
    context::Context,
    jrpc::{
        eth_types::{
            filter::{Filter, FilterChanges},
            log::Log,
            number::Number,
        },
        Error,
    },
};

pub fn new_filter(filter: Filter, context: &mut Context) -> Result<Number, Error> {
    let filters = context.evm_filters();
    Ok(filters.insert(FilterType::Log(filter)))
}

pub fn new_block_filter(context: &mut Context) -> Result<Number, Error> {
    let filters = context.evm_filters();
    Ok(filters.insert(FilterType::Block))
}

pub fn new_pending_transaction_filter(context: &mut Context) -> Result<Number, Error> {
    let filters = context.evm_filters();
    Ok(filters.insert(FilterType::PendingTransaction))
}

pub fn uninstall_filter(filter_id: Number, context: &mut Context) -> Result<bool, Error> {
    let filters = context.evm_filters();
    Ok(filters.remove(&filter_id))
}

pub fn get_filter_changes(_filter_id: Number, _context: &Context) -> Result<FilterChanges, Error> {
    Err(Error::NonArchiveNode)
}

pub fn get_filter_logs(_filter_id: Number, _context: &Context) -> Result<Vec<Log>, Error> {
    Err(Error::NonArchiveNode)
}

pub fn get_logs(_filter: Filter, _context: &Context) -> Result<FilterChanges, Error> {
    Err(Error::NonArchiveNode)
}
