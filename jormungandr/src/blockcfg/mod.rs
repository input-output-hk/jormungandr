pub use chain_impl_mockchain::{
    block::{builder as block_builder, Block},
    chaineval::HeaderContentEvalContext,
    chaintypes::ConsensusVersion,
    config::{self, Block0Date, ConfigParam},
    fragment::{ConfigParams, Contents, ContentsBuilder, Fragment, FragmentId},
    header::{
        BlockDate, BlockVersion, ChainLength, Epoch, Header, HeaderBft, HeaderBftBuilder,
        HeaderBuilder, HeaderBuilderNew, HeaderDesc, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
        HeaderId, HeaderSetConsensusSignature, SlotId,
    },
    leadership::{BftLeader, GenesisLeader, Leader, LeaderOutput, Leadership},
    ledger::{
        ApplyBlockLedger, EpochRewardsInfo, Ledger, LedgerStaticParameters, RewardsInfoParameters,
    },
    multiverse::Multiverse,
    value::{Value, ValueError},
};
pub use chain_network::data::gossip::Gossip;
use std::time::{Duration, SystemTime};
use thiserror::Error;

pub type HeaderHash = HeaderId;

#[derive(Debug, Error)]
pub enum Block0Error {
    #[error("Block0 Initial settings: {0}")]
    CannotParseEntity(#[from] config::Error),
    #[error("Block0 is invalid or malformed: {0}")]
    Malformed(#[from] Block0Malformed),
}

#[derive(Debug, Error)]
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
    for fragment in block.fragments() {
        if let Fragment::Initial(init) = fragment {
            return Ok(init);
        }
    }
    Err(Block0Malformed::NoInitialSettings)
}
