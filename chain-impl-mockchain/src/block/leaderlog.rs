use crate::certificate::PoolId;
use imhamt::Hamt;
use std::collections::hash_map::DefaultHasher;

/// Count how many blocks have been created by a specific Pool
#[derive(Clone)]
pub struct LeadersParticipationRecord {
    total: u32,
    log: Hamt<DefaultHasher, PoolId, u32>,
}

impl LeadersParticipationRecord {
    /// new empty leader log
    pub fn new() -> Self {
        Self {
            total: 0,
            log: Hamt::new(),
        }
    }

    /// Add one count to a pool. if the pool doesn't exist, then set it to 1
    pub fn increase_for(&self, pool: &PoolId) -> Self {
        Self {
            total: self.total + 1,
            log: self
                .log
                .insert_or_update_simple(pool.clone(), 1, |v| Some(v + 1)),
        }
    }
}
