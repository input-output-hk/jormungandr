use chain_core::property::HasMessages as _;
pub use network_core::gossip::Gossip;

pub use chain_impl_mockchain::{
    block::{
        Block, BlockBuilder, BlockDate, ChainLength, ConsensusVersion, Epoch, Header,
        HeaderContentEvalContext, HeaderHash, SlotId,
    },
    config::{self, Block0Date, ConfigParam},
    fragment::{ConfigParams, Fragment, FragmentId},
    leadership::{BftLeader, GenesisLeader, Leader, LeaderOutput, Leadership},
    ledger::{Ledger, LedgerParameters, LedgerStaticParameters},
    multiverse::Multiverse,
    value::{Value, ValueError},
};
use std::time::{Duration, SystemTime};

custom_error! {pub Block0Error
    CannotParseEntity{source: config::Error} = "Block0 Initial settings: {source}",
    Malformed{source: Block0Malformed} = "Block0 is invalid or malformed: {source}"
}

custom_error! {pub Block0Malformed
    NoInitialSettings = "missing its initial settings",
    NoStartTime = "missing `block0-start' value in the block0",
    NoDiscrimination = "missing `discrimination' value in the block0",
    NoSlotDuration = "missing `slot_duration' value in the block0",
    NoSlotsPerEpoch = "missing `slots_per_epoch' value in the block0",
}

pub trait Block0DataSource {
    fn slot_duration(&self) -> Result<Duration, Block0Error>;
    fn slots_per_epoch(&self) -> Result<u32, Block0Error>;
    fn start_time(&self) -> Result<SystemTime, Block0Error>;
}

impl Block0DataSource for Block {
    fn slot_duration(&self) -> Result<Duration, Block0Error> {
        for config in initial(self)?.iter() {
            if let ConfigParam::SlotDuration(duration) = config {
                return Ok(Duration::from_secs(*duration as u64));
            }
        }
        Err(Block0Malformed::NoSlotDuration.into())
    }

    fn slots_per_epoch(&self) -> Result<u32, Block0Error> {
        for config in initial(self)?.iter() {
            if let ConfigParam::SlotsPerEpoch(slots) = config {
                return Ok(*slots);
            }
        }
        Err(Block0Malformed::NoSlotsPerEpoch.into())
    }

    fn start_time(&self) -> Result<SystemTime, Block0Error> {
        for config in initial(self)?.iter() {
            if let ConfigParam::Block0Date(date) = config {
                return Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(date.0));
            }
        }
        Err(Block0Malformed::NoStartTime.into())
    }
}

fn initial(block: &Block) -> Result<&ConfigParams, Block0Malformed> {
    for message in block.messages() {
        if let Fragment::Initial(init) = message {
            return Ok(init);
        }
    }
    Err(Block0Malformed::NoInitialSettings)
}
