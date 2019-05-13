use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// The current transaction pool, containing all the transaction
/// that are potential for being inserted into a block, and their
/// received time
pub struct TPool<TransId, Trans> {
    pub content: HashMap<TransId, (SystemTime, Trans)>,
}

impl<TransId: std::hash::Hash + std::cmp::Eq, Trans: Clone> TPool<TransId, Trans> {
    /// Create a new pool
    pub fn new() -> Self {
        TPool {
            content: HashMap::new(),
        }
    }

    /// Check a transaction exist already in the pool
    pub fn exist(&self, id: &TransId) -> bool {
        self.content.contains_key(id)
    }

    /// Add a transaction into the pool
    pub fn add(&mut self, id: TransId, trans: Trans) {
        let t = SystemTime::now();
        // ignore the result
        let _ = self.content.insert(id, (t, trans));
        ()
    }

    pub fn get(&self, id: &TransId) -> Option<Trans> {
        self.content.get(id).map(|kv| kv.1.clone())
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
        let t = SystemTime::now();
        self.content.retain(|_, (received_time, _)| {
            t.duration_since(*received_time).unwrap() > expired_duration
        });
        orig_length - self.content.len()
    }
}
