use crate::block::{Block, Proof, BLOCK_VERSION_CONSENSUS_BFT};
use crate::leadership::{BftLeader, Error, ErrorKind, PublicLeader, Update};

use chain_core::property::{self, Block as _, LeaderSelection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BftRoundRobinIndex(u64);

/// The BFT Leader selection is based on a round robin of the expected leaders
#[derive(Debug)]
pub struct BftLeaderSelection {
    leaders: Vec<BftLeader>,

    current_leader: BftLeader,
}

#[derive(Debug, PartialEq)]
pub enum BftError {
    BlockHasInvalidLeader(PublicLeader, PublicLeader),
    BlockSignatureIsInvalid,
    BlockProofIsDifferent,
    UpdateHasInvalidCurrentLeader(PublicLeader, PublicLeader),
}

impl BftLeaderSelection {
    /// Create a new BFT leadership
    pub fn new(leaders: Vec<BftLeader>) -> Option<Self> {
        if leaders.len() == 0 {
            return None;
        }

        let current_leader = leaders[0].clone();
        Some(BftLeaderSelection {
            leaders: leaders,
            current_leader: current_leader,
        })
    }

    #[inline]
    pub fn number_of_leaders(&self) -> usize {
        self.leaders.len()
    }

    #[inline]
    fn offset(&self, block_number: u64) -> BftRoundRobinIndex {
        let max = self.number_of_leaders() as u64;
        BftRoundRobinIndex((block_number % max) as u64)
    }
}

impl LeaderSelection for BftLeaderSelection {
    type Block = Block;
    type Error = Error;
    type LeaderId = PublicLeader;

    #[inline]
    fn get_leader_at(
        &self,
        date: <Self::Block as property::Block>::Date,
    ) -> Result<Self::LeaderId, Self::Error> {
        let BftRoundRobinIndex(ofs) = self.offset(date.slot_id as u64);
        Ok(PublicLeader::Bft(self.leaders[ofs as usize].clone()))
    }
}

impl std::fmt::Display for BftError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BftError::BlockHasInvalidLeader(expected, found) => write!(
                f,
                "Invalid block leader, expected {:?} but the given block was signed by {:?}",
                expected, found
            ),
            BftError::BlockSignatureIsInvalid => write!(f, "The block signature is not valid"),
            BftError::UpdateHasInvalidCurrentLeader(current, found) => write!(
                f,
                "Update has an incompatible leader, we expect to update from {:?} but we are at {:?}",
                found, current
            ),
            BftError::BlockProofIsDifferent => write!(f, "The block proof is different and unexpected")
        }
    }
}
impl std::error::Error for BftError {}
