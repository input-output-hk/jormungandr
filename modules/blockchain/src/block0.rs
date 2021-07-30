use chain_impl_mockchain::{
    block::Block,
    config::{self, ConfigParam},
    fragment::{ConfigParams, Fragment},
};
use std::time::{Duration, SystemTime};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Block0Error {
    #[error("Block0 Initial settings: {0}")]
    CannotParseEntity(#[from] config::Error),
    #[error("Block0 is invalid or malformed: {0}")]
    Malformed(#[from] Block0Malformed),
}

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum Block0Malformed {
    #[error("missing its initial settings")]
    NoInitialSettings,
    #[error("missing `block0-start' value in the block0")]
    NoStartTime,
    #[error("missing `discrimination' value in the block0")]
    NoDiscrimination,
    #[error("missing `slot_duration' value in the block0")]
    NoSlotDuration,
    #[error("missing `slots_per_epoch' value in the block0")]
    NoSlotsPerEpoch,
}

pub fn slot_duration(block0: &Block) -> Result<Duration, Block0Error> {
    for config in initial(block0)?.iter() {
        if let ConfigParam::SlotDuration(duration) = config {
            return Ok(Duration::from_secs(*duration as u64));
        }
    }
    Err(Block0Malformed::NoSlotDuration.into())
}

pub fn start_time(block0: &Block) -> Result<SystemTime, Block0Error> {
    for config in initial(block0)?.iter() {
        if let ConfigParam::Block0Date(date) = config {
            return Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(date.0));
        }
    }
    Err(Block0Malformed::NoStartTime.into())
}

fn initial(block0: &Block) -> Result<&ConfigParams, Block0Malformed> {
    for fragment in block0.fragments() {
        if let Fragment::Initial(init) = fragment {
            return Ok(init);
        }
    }
    Err(Block0Malformed::NoInitialSettings)
}
