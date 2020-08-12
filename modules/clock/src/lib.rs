use chain_time::{
    era::{Epoch, EpochPosition, EpochSlotOffset},
    Slot, TimeEra, TimeFrame,
};
use std::time::{Duration, SystemTime};

/// define a `Clock` object, responsible for managing all that is time related
///
/// A Clock is only valid within a given TimeFrame and TimeEra.
/// The TimeFrame defines the relation between the blockchain and the time, the
/// Era defines how the blockchain time is split into epochs
pub struct Clock {
    frame: TimeFrame,
    era: TimeEra,
}

impl Clock {
    pub fn new(frame: TimeFrame, era: TimeEra) -> Self {
        Self { frame, era }
    }

    /// returns the current system time in the given clock.
    ///
    /// TODO: for testing purpose, it would be interesting to be able to
    ///       mock/update/configure the clock in a way the time can run
    ///       at different pace for testing and simulation
    #[inline]
    pub fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    #[inline]
    pub fn slot_duration(&self) -> Duration {
        Duration::from_secs(self.frame.slot_duration())
    }

    #[inline]
    pub fn slots_per_epoch(&self) -> usize {
        self.era.slots_per_epoch() as usize
    }

    #[inline]
    fn current_slot(&self) -> Option<Slot> {
        self.frame.slot_at(&self.now())
    }

    #[inline]
    fn next_slot(&self) -> Option<Slot> {
        let current: u64 = self.current_slot()?.into();
        let next = current + 1;
        Some(next.into())
    }

    /// get the current epoch position (epoch number and offset within this epoch)
    ///
    #[inline]
    pub fn current_epoch_position(&self) -> Option<EpochPosition> {
        let slot = self.current_slot()?;

        self.era.from_slot_to_era(slot)
    }

    /// get the next epoch position (epoch number and offset within this epoch)
    #[inline]
    pub fn next_epoch_position(&self) -> Option<EpochPosition> {
        let slot = self.next_slot()?;
        self.era.from_slot_to_era(slot)
    }

    /// get the system time to the next Epoch
    #[inline]
    pub fn next_epoch(&self) -> Option<EpochPosition> {
        let current = self.current_epoch_position()?;
        Some(EpochPosition {
            epoch: Epoch(current.epoch.0 + 1),
            slot: EpochSlotOffset(0),
        })
    }

    /// get the system time to the next Epoch
    #[inline]
    pub fn next_epoch_time(&self) -> Option<SystemTime> {
        let next_epoch = self.next_epoch()?;
        let slot = self.era.from_era_to_slot(next_epoch);
        self.frame.slot_to_systemtime(slot)
    }

    /// synchronously await for the next slot to start
    ///
    /// This function will block the current thread until the next slot is starting
    pub fn tick(&self) -> Option<()> {
        let dur = self.frame.slot_at_precise(&self.now())?.offset;

        std::thread::sleep(dur);
        Some(())
    }

    /// same as the `tick` function, but asynchronous (tokio::time)
    ///
    pub async fn tick_async(&self) -> Option<()> {
        let duration = self.frame.slot_at_precise(&self.now())?.offset;

        tokio::time::delay_for(duration).await;

        Some(())
    }
}
