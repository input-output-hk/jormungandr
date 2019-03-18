//! define the Blockchain settings
//!

use crate::update::ValueDiff;
use crate::{
    block::{BlockVersion, BLOCK_VERSION_CONSENSUS_NONE},
    key::Hash,
    leadership::bft,
};
use chain_core::property::{self, BlockId};

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
}

impl UpdateProposal {
    pub fn new() -> Self {
        UpdateProposal {
            max_number_of_transactions_per_block: None,
            bootstrap_key_slots_percentage: None,
            block_version: None,
            bft_leaders: None,
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
                None => panic!("Unrecognized update tag {}.", tag),
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub last_block_id: Hash,
    pub max_number_of_transactions_per_block: u32,
    pub bootstrap_key_slots_percentage: u8, // == d * 100
    pub block_version: BlockVersion,
    pub bft_leaders: Vec<bft::LeaderId>,
}

pub const SLOTS_PERCENTAGE_RANGE: u8 = 100;

impl Settings {
    pub fn new() -> Self {
        Self {
            last_block_id: Hash::zero(),
            max_number_of_transactions_per_block: 100,
            bootstrap_key_slots_percentage: SLOTS_PERCENTAGE_RANGE,
            block_version: BLOCK_VERSION_CONSENSUS_NONE,
            bft_leaders: Vec::new(),
        }
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
    type Error = Error;
    type Block = crate::block::Block;

    fn tip(&self) -> <Self::Block as property::Block>::Id {
        self.last_block_id.clone()
    }

    fn max_number_of_transactions_per_block(&self) -> u32 {
        self.max_number_of_transactions_per_block
    }

    fn block_version(&self) -> <Self::Block as property::Block>::Version {
        self.block_version
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SettingsDiff {
    pub block_id: ValueDiff<Hash>,
    pub bootstrap_key_slots_percentage: ValueDiff<u8>,
    pub block_version: ValueDiff<BlockVersion>,
    pub bft_leaders: ValueDiff<Vec<bft::LeaderId>>,
}

impl property::Update for SettingsDiff {
    fn empty() -> Self {
        SettingsDiff {
            block_id: ValueDiff::None,
            bootstrap_key_slots_percentage: ValueDiff::None,
            block_version: ValueDiff::None,
            bft_leaders: ValueDiff::None,
        }
    }
    fn inverse(self) -> Self {
        SettingsDiff {
            block_id: self.block_id.inverse(),
            bootstrap_key_slots_percentage: self.bootstrap_key_slots_percentage.inverse(),
            block_version: self.block_version.inverse(),
            bft_leaders: self.bft_leaders.inverse(),
        }
    }
    fn union(&mut self, other: Self) -> &mut Self {
        self.block_id.union(other.block_id);
        self.bootstrap_key_slots_percentage
            .union(other.bootstrap_key_slots_percentage);
        self.block_version.union(other.block_version);
        self.bft_leaders.union(other.bft_leaders);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chain_core::property::testing;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for SettingsDiff {
        fn arbitrary<G: Gen>(g: &mut G) -> SettingsDiff {
            SettingsDiff {
                block_version: ValueDiff::None,
                block_id: ValueDiff::Replace(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g)),
                bootstrap_key_slots_percentage: ValueDiff::Replace(
                    Arbitrary::arbitrary(g),
                    Arbitrary::arbitrary(g),
                ),
                bft_leaders: ValueDiff::None,
            }
        }
    }

    quickcheck! {
        /*
        FIXME: add tests for checking associativity of diffs on
        randomly generated values of the type we're diffing.

        fn settings_diff_union_is_associative(types: (SettingsDiff, SettingsDiff, SettingsDiff)) -> bool {
            testing::update_associativity(types.0, types.1, types.2)
        }
        */
        fn settings_diff_union_has_identity_element(settings_diff: SettingsDiff) -> bool {
            testing::update_identity_element(settings_diff)
        }
        fn settings_diff_union_has_inverse_element(settings_diff: SettingsDiff) -> bool {
            testing::update_inverse_element(settings_diff)
        }

    }
}
