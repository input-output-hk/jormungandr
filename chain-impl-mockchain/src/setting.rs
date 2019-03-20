//! define the Blockchain settings
//!

use crate::{
    block::{BlockDate, BlockId, BlockVersion, ChainLength, BLOCK_VERSION_CONSENSUS_NONE},
    key::Hash,
    leadership::bft,
};
use chain_core::property::{self, BlockId as _};
use std::sync::Arc;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

// FIXME: sign UpdateProposals, add voting, execute updates at an
// epoch boundary.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateProposal {
    pub max_number_of_transactions_per_block: Option<u32>,
    pub bootstrap_key_slots_percentage: Option<u8>,
    pub block_version: Option<BlockVersion>,
    pub bft_leaders: Option<Vec<bft::LeaderId>>,
    /// update to trigger allowing the creation of accounts without
    /// publishing a certificate
    pub allow_account_creation: Option<bool>,
}

impl UpdateProposal {
    pub fn new() -> Self {
        UpdateProposal {
            max_number_of_transactions_per_block: None,
            bootstrap_key_slots_percentage: None,
            block_version: None,
            bft_leaders: None,
            allow_account_creation: None,
        }
    }
}

#[derive(FromPrimitive)]
enum UpdateTag {
    End = 0,
    MaxNumberOfTransactionsPerBlock = 1,
    BootstrapKeySlotsPercentage = 2,
    BlockVersion = 3,
    BftLeaders = 4,
    AllowAccountCreation = 5,
}

impl property::Serialize for UpdateProposal {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        if let Some(max_number_of_transactions_per_block) =
            self.max_number_of_transactions_per_block
        {
            codec.put_u16(UpdateTag::MaxNumberOfTransactionsPerBlock as u16)?;
            codec.put_u32(max_number_of_transactions_per_block)?;
        }
        if let Some(bootstrap_key_slots_percentage) = self.bootstrap_key_slots_percentage {
            codec.put_u16(UpdateTag::BootstrapKeySlotsPercentage as u16)?;
            codec.put_u8(bootstrap_key_slots_percentage)?;
        }
        if let Some(block_version) = self.block_version {
            codec.put_u16(UpdateTag::BlockVersion as u16)?;
            codec.put_u16(block_version.0)?;
        }
        if let Some(leaders) = &self.bft_leaders {
            codec.put_u16(UpdateTag::BftLeaders as u16)?;
            codec.put_u8(leaders.len() as u8)?;
            for leader in leaders.iter() {
                leader.serialize(&mut codec)?;
            }
        }
        if let Some(allow_account_creation) = &self.allow_account_creation {
            codec.put_u16(UpdateTag::AllowAccountCreation as u16)?;
            codec.put_u8(if *allow_account_creation { 1 } else { 0 })?;
        }
        codec.put_u16(UpdateTag::End as u16)?;
        Ok(())
    }
}

impl property::Deserialize for UpdateProposal {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        let mut update = UpdateProposal::new();
        loop {
            let tag = codec.get_u16()?;
            match UpdateTag::from_u16(tag) {
                Some(UpdateTag::End) => {
                    return Ok(update);
                }
                Some(UpdateTag::MaxNumberOfTransactionsPerBlock) => {
                    update.max_number_of_transactions_per_block = Some(codec.get_u32()?);
                }
                Some(UpdateTag::BootstrapKeySlotsPercentage) => {
                    update.bootstrap_key_slots_percentage = Some(codec.get_u8()?);
                }
                Some(UpdateTag::BlockVersion) => {
                    update.block_version = Some(codec.get_u16().map(BlockVersion)?);
                }
                Some(UpdateTag::BftLeaders) => {
                    let len = codec.get_u8()? as usize;
                    let mut leaders = Vec::with_capacity(len);
                    for _ in 0..len {
                        leaders.push(bft::LeaderId::deserialize(&mut codec)?);
                    }
                    update.bft_leaders = Some(leaders);
                }
                Some(UpdateTag::AllowAccountCreation) => {
                    let boolean = codec.get_u8()? != 0;
                    update.allow_account_creation = Some(boolean);
                }
                None => panic!("Unrecognized update tag {}.", tag),
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub last_block_id: BlockId,
    pub last_block_date: BlockDate,
    pub chain_length: ChainLength,
    pub max_number_of_transactions_per_block: Arc<u32>,
    pub bootstrap_key_slots_percentage: Arc<u8>, // == d * 100
    pub block_version: Arc<BlockVersion>,
    pub bft_leaders: Arc<Vec<bft::LeaderId>>,
    /// allow for the creation of accounts without the certificate
    pub allow_account_creation: Arc<bool>,
}

pub const SLOTS_PERCENTAGE_RANGE: u8 = 100;

impl Settings {
    pub fn new() -> Self {
        Self {
            last_block_id: Hash::zero(),
            last_block_date: BlockDate::first(),
            chain_length: ChainLength(0),
            max_number_of_transactions_per_block: Arc::new(100),
            bootstrap_key_slots_percentage: Arc::new(SLOTS_PERCENTAGE_RANGE),
            block_version: Arc::new(BLOCK_VERSION_CONSENSUS_NONE),
            bft_leaders: Arc::new(Vec::new()),
            allow_account_creation: Arc::new(false),
        }
    }

    pub fn allow_account_creation(&self) -> bool {
        *self.allow_account_creation
    }

    pub fn apply(&self, update: UpdateProposal) -> Self {
        let mut new_state = self.clone();
        if let Some(max_number_of_transactions_per_block) =
            update.max_number_of_transactions_per_block
        {
            new_state.max_number_of_transactions_per_block =
                Arc::new(max_number_of_transactions_per_block);
        }
        if let Some(bootstrap_key_slots_percentage) = update.bootstrap_key_slots_percentage {
            new_state.bootstrap_key_slots_percentage = Arc::new(bootstrap_key_slots_percentage);
        }
        if let Some(block_version) = update.block_version {
            new_state.block_version = Arc::new(block_version);
        }
        if let Some(leaders) = update.bft_leaders {
            new_state.bft_leaders = Arc::new(leaders);
        }
        if let Some(allow_account_creation) = update.allow_account_creation {
            new_state.allow_account_creation = Arc::new(allow_account_creation);
        }
        new_state
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidCurrentBlockId(Hash, Hash),
    UpdateIsInvalid,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InvalidCurrentBlockId(current_one, update_one) => {
                write!(f, "Cannot apply Setting Update. Update needs to be applied to from block {:?} but received {:?}", update_one, current_one)
            }
            Error::UpdateIsInvalid => write!(
                f,
                "Update does not apply to current state"
            ),
        }
    }
}
impl std::error::Error for Error {}

impl property::Settings for Settings {
    type Block = crate::block::Block;

    fn tip(&self) -> <Self::Block as property::Block>::Id {
        self.last_block_id.clone()
    }

    fn max_number_of_transactions_per_block(&self) -> u32 {
        *self.max_number_of_transactions_per_block
    }

    fn block_version(&self) -> <Self::Block as property::Block>::Version {
        *self.block_version
    }

    fn chain_length(&self) -> <Self::Block as property::Block>::ChainLength {
        self.chain_length
    }
}
