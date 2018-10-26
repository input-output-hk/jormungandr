pub mod configuration;
pub mod global;

use std::thread;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use self::configuration::{Epoch};

pub use self::configuration::{ClockEpochConfiguration, ClockConfiguration};

#[derive(Clone)]
pub struct Current {
    epoch: configuration::Epoch,
    slot: usize,
}

impl Current {
    pub fn new() -> Self {
        Current {
            epoch: Epoch(0),
            slot: 0,
        }
    }
    pub fn next(&self, cfg: &ClockEpochConfiguration) -> Self {
        let next_slot = self.slot + 1;
        if next_slot == cfg.slots_per_epoch {
            Current { epoch: self.epoch.next(), slot: 0 }
        } else {
            Current { epoch: self.epoch, slot: next_slot }
        }
    }
}

#[derive(Clone)]
pub struct Clock {
    configuration: Arc<ClockConfiguration>,
    current_slot: Arc<RwLock<Current>>,
}

impl Clock {
    pub fn new(config: ClockConfiguration) -> Self {
        Clock {
            configuration: Arc::new(config),
            current_slot: Arc::new(RwLock::new(Current::new()))
        }
    }

    pub fn advance_slot(&self) {
        let ecfg = self.configuration.get_latest_configuration();
        let mut cs = self.current_slot.write().unwrap();
        *cs = (*cs).next(&ecfg)
    }

    pub fn wait_next_slot(&self) {
        // TODO: we need to wait the difference of the current time, minus the current slot
        thread::sleep(Duration::from_secs(20))
    }
}
