use std::collections::HashMap;
use clock::global::GlobalTime;
use std::time::Duration;

/// The current transaction pool, containing all the transaction
/// that are potential for being inserted into a block, and their
/// received time
pub struct TPool<TransId, Trans> {
    pub content: HashMap<TransId, (GlobalTime, Trans)>,
}

impl<TransId: std::hash::Hash+std::cmp::Eq, Trans> TPool<TransId, Trans> {
    /// Create a new pool
    pub fn new() -> Self {
        TPool { content: HashMap::new() }
    }

    /// Check a transaction exist already in the pool
    pub fn exist(&self, id: &TransId) -> bool {
        self.content.contains_key(id)
    }

    /// Add a transaction into the pool
    pub fn add(&mut self, id: TransId, trans: Trans) {
        let t = GlobalTime::now();
        // ignore the result
        let _ = self.content.insert(id, (t, trans));
        ()
    }

    /// remove the `count` transaction from the pool
    pub fn collect(&mut self, count: usize) -> Vec<Trans> {
        let content = std::mem::replace(&mut self.content, HashMap::new());
        let mut selected = Vec::with_capacity(count);

        for (index, kv) in content.into_iter().enumerate() {
            if index < count {
                selected.push((kv.1).1);
            } else {
                self.content.insert(kv.0, kv.1);
            }
        }

        selected
    }

    /// Garbage collect all the necessary transactions
    pub fn gc(&mut self, expired_duration: Duration) -> usize {
        let orig_length = self.content.len();
        let t = GlobalTime::now();
        self.content.retain(|_, (ttime, _)| t.differential(*ttime) > expired_duration);
        orig_length - self.content.len()
    }
}
