use crate::common::jormungandr::{JormungandrProcess, JormungandrRest};
use chain_impl_mockchain::fee::LinearFee;
use jormungandr_lib::crypto::hash::Hash;
pub struct Node {
    jormungandr: JormungandrProcess,
    alias: String,
}

impl Node {
    pub fn new(jormungandr: JormungandrProcess, alias: &str) -> Self {
        Self {
            jormungandr: jormungandr,
            alias: alias.to_string(),
        }
    }

    pub fn alias(&self) -> String {
        self.alias.to_string()
    }

    pub fn rest(&self) -> JormungandrRest {
        self.jormungandr.rest()
    }

    pub fn assert_no_errors_in_log(&self) {
        self.jormungandr.assert_no_errors_in_log();
    }

    pub fn address(&self) -> poldercast::Address {
        self.jormungandr
            .config
            .node_config()
            .p2p
            .public_address
            .clone()
    }

    pub fn shutdown(&self) {
        self.jormungandr.shutdown();
    }

    pub fn genesis_block_hash(&self) -> Hash {
        self.jormungandr.genesis_block_hash()
    }

    pub fn fees(&self) -> LinearFee {
        self.jormungandr.fees()
    }

    pub fn process(&self) -> &JormungandrProcess {
        &self.jormungandr
    }
}

impl Into<JormungandrProcess> for Node {
    fn into(self) -> JormungandrProcess {
        self.jormungandr
    }
}
