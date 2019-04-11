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
#[derive(Debug, Clone)]
pub struct TimeEra {
    epoch_start: Epoch,
    slot_start: Slot,
    slots_per_epoch: u32,
}

impl TimeEra {
    /// Set a new era to start on slot_start at epoch_start for a given slots per epoch.
    pub fn new_era(slot_start: Slot, epoch_start: Epoch, slots_per_epoch: u32) -> Self {
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
