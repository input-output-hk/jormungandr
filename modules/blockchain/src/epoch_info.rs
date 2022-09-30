use crate::block0;
use chain_impl_mockchain::{
    block::Block,
    header::{BlockDate, Header},
    leadership::{self, Leadership, Verification},
    ledger::{EpochRewardsInfo, Ledger},
};
use chain_time::{
    era::{EpochPosition, EpochSlotOffset},
    Epoch, Slot, TimeFrame,
};
use std::time::SystemTime;
use thiserror::Error;

pub struct EpochInfo {
    /// the time frame applicable in the current branch of the blockchain
    time_frame: TimeFrame,
    /// the leadership used to validate the current header's leader
    ///
    /// this object will be shared between different Ref of the same epoch
    epoch_leadership_schedule: Leadership,

    /// If present, this is the rewards info distributed at the beginning of
    /// the epoch. Useful to follow up on the reward distribution history
    epoch_rewards_info: Option<EpochRewardsInfo>,
}

#[derive(Debug, Error)]
pub enum EpochInfoError {
    #[error("Cannot needed information from the block0")]
    InvalidBlock0(
        #[source]
        #[from]
        block0::Block0Error,
    ),

    #[error("Block Header's verification failed")]
    HeaderVerification {
        #[source]
        #[from]
        source: leadership::Error,
    },
}

impl EpochInfo {
    pub(crate) fn new(block0: &Block, ledger: &Ledger) -> Result<Self, EpochInfoError> {
        let epoch = block0.header().block_date().epoch;
        let time_frame = {
            let start_time = block0::start_time(block0)?;
            let slot_duration = block0::slot_duration(block0)?;

            TimeFrame::new(
                chain_time::Timeline::new(start_time),
                chain_time::SlotDuration::from_secs(slot_duration.as_secs() as u32),
            )
        };

        let epoch_leadership_schedule = Leadership::new(epoch, ledger);
        let epoch_rewards_info = None;

        Ok(Self {
            time_frame,
            epoch_leadership_schedule,
            epoch_rewards_info,
        })
    }

    pub(crate) fn chain(
        &self,
        leadership: Leadership,
        epoch_rewards_info: Option<EpochRewardsInfo>,
    ) -> Self {
        Self {
            time_frame: self.time_frame.clone(),
            epoch_leadership_schedule: leadership,
            epoch_rewards_info,
        }
    }

    pub fn check_header(&self, header: &Header) -> Result<(), EpochInfoError> {
        match self.epoch_leadership_schedule.verify(header) {
            Verification::Failure(error) => {
                Err(EpochInfoError::HeaderVerification { source: error })
            }
            Verification::Success => Ok(()),
        }
    }

    pub fn epoch(&self) -> u32 {
        self.epoch_leadership_schedule.epoch()
    }

    /// get the slot for the given BlockDate
    ///
    /// Having this available allows for better handling of the time scheduled
    /// of the given date.
    pub fn slot_of(&self, date: BlockDate) -> Slot {
        let epoch = Epoch(date.epoch);
        let slot = EpochSlotOffset(date.slot_id);

        let pos = EpochPosition { epoch, slot };

        self.epoch_leadership_schedule.era().from_era_to_slot(pos)
    }

    /// get the system time scheduled for the given block date
    ///
    /// This function returns `None` if the block date is not
    /// within the time frame of this EpochInfo (i.e. the time
    /// frame has changed)
    pub fn time_of(&self, date: BlockDate) -> Option<SystemTime> {
        let slot = self.slot_of(date);

        self.time_frame.slot_to_systemtime(slot)
    }

    pub fn epoch_leadership_schedule(&self) -> &Leadership {
        &self.epoch_leadership_schedule
    }

    /// access the rewards info that were distributed at the end of the previous epoch
    /// (and that are accessible/visible from this epoch only).
    pub fn epoch_rewards_info(&self) -> Option<&EpochRewardsInfo> {
        self.epoch_rewards_info.as_ref()
    }
}
