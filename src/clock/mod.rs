pub mod configuration;
pub mod global;

use self::configuration::Epoch;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

pub use self::configuration::ClockEpochConfiguration;

pub struct FlatSlotId(u64);

#[derive(Clone)]
pub struct Clock {
    initial_time: SystemTime,
    initial_configuration: ClockEpochConfiguration,
    eras: Arc<RwLock<Vec<(SystemTime, Epoch, ClockEpochConfiguration)>>>,
}

impl Clock {
    pub fn new(initial_time: SystemTime, config: ClockEpochConfiguration) -> Self {
        Clock {
            initial_time: initial_time,
            initial_configuration: config,
            eras: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn get_era_at(&self, at: &SystemTime) -> Option<(Duration, Epoch, ClockEpochConfiguration)> {
        if at < &self.initial_time {
            None
        } else {
            let eras = self.eras.read().unwrap();
            for (era_st, e, cfg) in eras.iter().rev() {
                if era_st < at {
                    match at.duration_since(*era_st) {
                        Err(_) => {}
                        Ok(d) => return Some((d, *e, cfg.clone())),
                    }
                }
            }
            match at.duration_since(self.initial_time) {
                Err(_) => None,
                Ok(d) => Some((d, Epoch(0), self.initial_configuration.clone())),
            }
        }
    }

    fn get_last_era(&self) -> (SystemTime, Epoch, ClockEpochConfiguration) {
        let e = self.eras.read().unwrap();
        (*e).last().map(|t| t.clone()).unwrap_or((
            self.initial_time,
            Epoch(0),
            self.initial_configuration.clone(),
        ))
    }

    pub fn append_era(&self, epoch: Epoch, config: ClockEpochConfiguration) {
        // get the latest era configuration
        let (previous_era_time, previous_era_epoch, previous_cfg) = self.get_last_era();

        assert!(epoch > previous_era_epoch);

        let epoch_duration = previous_cfg.epoch_duration();
        let epoch_diff = epoch.0 - previous_era_epoch.0;
        let new_era_time = previous_era_time + epoch_duration * epoch_diff;
        let append_era = (new_era_time, epoch, config);

        let mut eras = self.eras.write().unwrap();
        eras.push(append_era)
    }

    /// Return the slot id and the remaining duration of this slot
    fn current_slot_at(&self, at: &SystemTime) -> Option<(Epoch, u32, Duration)> {
        match self.get_era_at(at) {
            None => None,
            Some((time_offset_in_era, era_epoch, era_cfg)) => {
                let flat_slot_index =
                    time_offset_in_era.as_secs() / era_cfg.slot_duration.as_secs();
                let epoch =
                    Epoch(era_epoch.0 + (flat_slot_index / era_cfg.slots_per_epoch as u64) as u32);
                let slot = (flat_slot_index as u32) % (era_cfg.slots_per_epoch as u32);

                let next_slot_time = era_cfg.slot_duration * ((flat_slot_index + 1) as u32);

                Some((epoch, slot, next_slot_time - time_offset_in_era))
            }
        }
    }

    pub fn current_slot(&self) -> Option<(Epoch, u32, Duration)> {
        self.current_slot_at(&SystemTime::now())
    }

    pub fn wait_next_slot(&self) -> Option<Duration> {
        // could just calculate the duration
        self.current_slot().map(|(_, _, d)| {
            thread::sleep(d);
            d
        })
    }
}
