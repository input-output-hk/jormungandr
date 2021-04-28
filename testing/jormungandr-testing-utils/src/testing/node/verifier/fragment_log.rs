use crate::testing::node::rest::RestError;
use crate::testing::node::JormungandrRest;
use chain_impl_mockchain::fragment::FragmentId;
use chain_impl_mockchain::key::Hash;
use jormungandr_lib::interfaces::FragmentLog;
use std::collections::HashMap;

pub struct FragmentLogVerifier {
    fragment_logs: HashMap<FragmentId, FragmentLog>,
}

impl FragmentLogVerifier {
    pub fn new(rest: JormungandrRest) -> Result<Self, RestError> {
        Ok(Self {
            fragment_logs: rest.fragment_logs()?,
        })
    }

    pub fn assert_size(self, size: usize) -> Self {
        assert_eq!(
            self.fragment_logs.len(),
            size,
            "only 1 transaction should be in fragment log"
        );
        self
    }

    pub fn contains_only(self, hash: &Hash) -> Self {
        assert_eq!(
            self.fragment_logs
                .values()
                .next()
                .unwrap()
                .fragment_id()
                .into_hash(),
            *hash,
            "transaction not found in fragment log"
        );
        self
    }

    pub fn assert_empty(self) -> Self {
        assert_eq!(
            self.fragment_logs.len(),
            0,
            "none transactions should be in fragment log"
        );
        self
    }
}
