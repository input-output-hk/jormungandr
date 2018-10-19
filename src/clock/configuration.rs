use std::time::Duration;
use std::sync::{Arc,RwLock};

/// epochs. TODO figure out if reusing the epoch from cardano make sense
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Epoch(pub u32);

impl Epoch {
    pub fn next(&self) -> Self {
        Epoch(self.0 + 1)
    }
}

/// Potentially one per epoch, one given by default by configuration
#[derive(Clone)]
pub struct ClockEpochConfiguration {
    pub slot_duration: Duration,
    pub slots_per_epoch: usize,
}

pub struct ClockConfiguration {
    initial: ClockEpochConfiguration,
    updates: Arc<RwLock<Vec<(Epoch, ClockEpochConfiguration)>>>,
}

impl ClockConfiguration {
    pub fn new(initial: ClockEpochConfiguration) -> Self {
        ClockConfiguration {
            initial: initial,
            updates: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn push_configuration(&self, epoch_era: Epoch, cfg: ClockEpochConfiguration) {
        let mut u = self.updates.write().unwrap();
        (*u).push((epoch_era, cfg))
    }

    pub fn get_epoch_configuration(&self, epoch_era: Epoch) -> ClockEpochConfiguration {
        let updates = self.updates.read().unwrap();
        for (e, cec) in (*updates).iter().rev() {
            if &epoch_era >= e {
                return cec.clone();
            }
        }
        return self.initial.clone();
    }

    pub fn get_latest_configuration(&self) -> ClockEpochConfiguration {
        let updates = self.updates.read().unwrap();
        let len = (*updates).len();
        if len > 0 {
            return (*updates)[len - 1].1.clone()
        } else {
            return self.initial.clone()
        }
    }
}
