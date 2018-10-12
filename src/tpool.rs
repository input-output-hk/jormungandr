use std::collections::BTreeMap;
//use cardano::tx::{TxId, TxAux};

/// The current transaction pool, containing all the transaction
/// that are potential for being inserted into a block
pub struct TPool<TransId, Trans> {
    content: BTreeMap<TransId, Trans>,
}

impl<TransId: std::cmp::Ord, Trans> TPool<TransId, Trans> {
    /// Create a new pool
    pub fn new() -> Self {
        TPool { content: BTreeMap::new() }
    }

    /// Check a transaction exist already in the pool
    pub fn exist(&self, id: &TransId) -> bool {
        self.content.contains_key(id)
    }

    /// Add a transaction into the pool
    pub fn add(&mut self, id: TransId, trans: Trans) {
        // ignore the result
        let _ = self.content.insert(id, trans);
        ()
    }
}
