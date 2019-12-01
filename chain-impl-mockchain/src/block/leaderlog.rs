use crate::certificate::PoolId;
use imhamt::{Hamt, HamtIter, InsertError};
use std::collections::hash_map::DefaultHasher;

/// Count how many blocks have been created by a specific Pool
#[derive(Clone, PartialEq, Eq)]
pub struct LeadersParticipationRecord {
    total: u32,
    log: Hamt<DefaultHasher, PoolId, u32>,
}

impl LeadersParticipationRecord {
    pub fn total(&self) -> u32 {
        self.total
    }

    /// new empty leader log
    pub fn new() -> Self {
        Self {
            total: 0,
            log: Hamt::new(),
        }
    }

    /// Add one count to a pool. if the pool doesn't exist, then set it to 1
    pub fn increase_for(&mut self, pool: &PoolId) {
        self.total = self.total + 1;
        self.log = self
            .log
            .insert_or_update_simple(pool.clone(), 1, |v| Some(v + 1));
    }

    /// Set a pool id to a specific value.
    ///
    /// if the value already exists, then it returns an insert error.
    /// This should only be used related to the iterator construction,
    pub fn set_for(&mut self, pool: PoolId, v: u32) -> Result<(), InsertError> {
        self.total += v;
        self.log = self.log.insert(pool, v)?;
        Ok(())
    }

    /// Iterate over all known pool record
    pub fn iter<'a>(&'a self) -> HamtIter<'a, PoolId, u32> {
        self.log.iter()
    }
}
