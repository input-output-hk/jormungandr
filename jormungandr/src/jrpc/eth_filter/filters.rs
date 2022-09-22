use crate::jrpc::eth_types::{filter::Filter, number::Number};
use std::collections::HashMap;

#[derive(Default)]
pub struct EvmFilters {
    last_key: Number,
    filters: HashMap<Number, FilterType>,
}

impl EvmFilters {
    pub fn insert(&mut self, filter: FilterType) -> Number {
        self.last_key.inc();
        self.filters.insert(self.last_key.clone(), filter);
        self.last_key.clone()
    }

    pub fn get(&self, index: &Number) -> Option<&FilterType> {
        self.filters.get(index)
    }

    pub fn remove(&mut self, index: &Number) -> bool {
        self.filters.remove(index).map_or_else(|| false, |_| true)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FilterType {
    Block,
    PendingTransaction,
    Log(Filter),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evm_filters_test() {
        let mut filters = EvmFilters::default();

        let index1 = filters.insert(FilterType::Block);
        let index2 = filters.insert(FilterType::PendingTransaction);

        assert_eq!(filters.get(&index1), Some(&FilterType::Block));
        assert_eq!(filters.get(&index2), Some(&FilterType::PendingTransaction));

        assert!(filters.remove(&index1));

        assert_eq!(filters.get(&index1), None);
        assert_eq!(filters.get(&index2), Some(&FilterType::PendingTransaction));

        assert!(!filters.remove(&index1));
    }
}
