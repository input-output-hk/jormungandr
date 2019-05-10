use std::time::Duration;

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
    pub slots_per_epoch: u32,
}

impl ClockEpochConfiguration {
    pub fn epoch_duration(&self) -> Duration {
        self.slot_duration * (self.slots_per_epoch as u32)
    }
}
