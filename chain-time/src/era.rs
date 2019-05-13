//! Split timeframe in eras

use crate::timeframe::Slot;

/// Epoch number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Epoch(pub u32);

/// Slot Offset *in* a given epoch
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EpochSlotOffset(pub u32);

/// Epoch position: this is an epoch and a slot offset
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EpochPosition {
    pub epoch: Epoch,
    pub slot: EpochSlotOffset,
}

/// Describe a new era, which start at epoch_start and is associated
/// to a specific slot. Each epoch have a constant number of slots on a given time era.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeEra {
    epoch_start: Epoch,
    slot_start: Slot,
    pub slots_per_epoch: u32,
}

impl TimeEra {
    /// Set a new era to start on slot_start at epoch_start for a given slots per epoch.
    pub fn new(slot_start: Slot, epoch_start: Epoch, slots_per_epoch: u32) -> Self {
        TimeEra {
            epoch_start,
            slot_start,
            slots_per_epoch,
        }
    }

    /// Try to return the epoch/inner-epoch-slot associated.
    ///
    /// If the slot in parameter is before the beginning of this era, then
    /// None is returned.
    pub fn from_slot_to_era(&self, slot: Slot) -> Option<EpochPosition> {
        if slot < self.slot_start {
            return None;
        }
        let slot_era_offset = slot.0 - self.slot_start.0;
        let spe = self.slots_per_epoch as u64;
        let epoch_offset = (slot_era_offset / spe) as u32;
        let slot_offset = (slot_era_offset % spe) as u32;
        Some(EpochPosition {
            epoch: Epoch(self.epoch_start.0 + epoch_offset),
            slot: EpochSlotOffset(slot_offset),
        })
    }

    /// Convert an epoch position into a flat slot
    pub fn from_era_to_slot(&self, pos: EpochPosition) -> Slot {
        assert!(pos.epoch >= self.epoch_start);
        assert!(pos.slot.0 < self.slots_per_epoch);

        let slot_offset = (pos.epoch.0 as u64) * (self.slots_per_epoch as u64) + pos.slot.0 as u64;
        Slot(self.slot_start.0 + slot_offset)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::timeframe::*;
    use crate::timeline::Timeline;
    use std::time::{Duration, SystemTime};

    #[test]
    pub fn it_works() {
        let now = SystemTime::now();
        let t0 = Timeline::new(now);

        let f0 = SlotDuration::from_secs(5);

        let tf0 = TimeFrame::new(t0, f0);

        let t1 = now + Duration::from_secs(10);
        let t2 = now + Duration::from_secs(20);
        let t3 = now + Duration::from_secs(100);

        let slot1 = tf0.slot_at(&t1).unwrap();
        let slot2 = tf0.slot_at(&t2).unwrap();
        let slot3 = tf0.slot_at(&t3).unwrap();

        assert_eq!(slot1, Slot(2));
        assert_eq!(slot2, Slot(4));
        assert_eq!(slot3, Slot(20));

        let era = TimeEra::new(slot1, Epoch(2), 4);

        let p1 = era.from_slot_to_era(slot1).unwrap();
        let p2 = era.from_slot_to_era(slot2).unwrap();
        let p3 = era.from_slot_to_era(slot3).unwrap();

        assert_eq!(
            p1,
            EpochPosition {
                epoch: Epoch(2),
                slot: EpochSlotOffset(0)
            }
        );
        assert_eq!(
            p2,
            EpochPosition {
                epoch: Epoch(2),
                slot: EpochSlotOffset(2)
            }
        );
        // 20 - 2 => 18 / 4 => era_start(2) + (4, 2)
        assert_eq!(
            p3,
            EpochPosition {
                epoch: Epoch(6),
                slot: EpochSlotOffset(2)
            }
        );
    }
}
