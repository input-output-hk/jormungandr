use crate::jrpc::eth_types::{filter::Filter, number::Number};
use std::collections::HashMap;

#[derive(Default)]
pub struct EvmFilters {
    filters: HashMap<Number, FilterType>,
}

impl EvmFilters {
    pub fn insert(&mut self, filter: FilterType) -> Number {
        let index: Number = ((self.filters.len() - 1) as u64).into();
        self.filters.insert(index.clone(), filter);
        index
    }

    pub fn get(&self, index: &Number) -> Option<&FilterType> {
        self.filters.get(index)
    }

    pub fn remove(&mut self, index: &Number) -> bool {
        self.filters.remove(index).map_or_else(|| false, |_| true)
    }
}

#[derive(Debug)]
pub enum FilterType {
    Block,
    PendingTransaction,
    Log(Filter),
}
