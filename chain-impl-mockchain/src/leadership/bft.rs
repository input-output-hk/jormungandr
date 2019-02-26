use crate::block::SignedBlock;
use crate::key::PublicKey;
use crate::update::{BftSelectionDiff, ValueDiff};

use chain_core::property::{self, LeaderSelection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BftRoundRobinIndex(usize);

/// The BFT Leader selection is based on a round robin of the expected leaders
#[derive(Debug)]
pub struct BftLeaderSelection<LeaderId> {
    leaders: Vec<LeaderId>,

    current_leader: LeaderId,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    BlockHasInvalidLeader(PublicKey, PublicKey),
    BlockSignatureIsInvalid,
    UpdateHasInvalidCurrentLeader(PublicKey, PublicKey),
}

impl<LeaderId: Eq + Clone> BftLeaderSelection<LeaderId> {
    /// Create a new BFT leadership
    pub fn new(leaders: Vec<LeaderId>) -> Option<Self> {
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
        BftRoundRobinIndex((block_number % max) as usize)
    }
}

impl chain_core::property::LeaderId for PublicKey {}

impl LeaderSelection for BftLeaderSelection<PublicKey> {
    type Update = BftSelectionDiff;
    type Block = SignedBlock;
    type Error = Error;
    type LeaderId = PublicKey;

    fn diff(&self, input: &Self::Block) -> Result<Self::Update, Self::Error> {
        use chain_core::property::Block;

        let mut update = <Self::Update as property::Update>::empty();

        let date = input.date();
        let new_leader = self.get_leader_at(date)?;

        if new_leader != input.public_key {
            return Err(Error::BlockHasInvalidLeader(
                new_leader,
                input.public_key.clone(),
            ));
        }
        if !input.verify() {
            return Err(Error::BlockSignatureIsInvalid);
        }

        update.leader = ValueDiff::Replace(self.current_leader.clone(), new_leader);

        Ok(update)
    }
    fn apply(&mut self, update: Self::Update) -> Result<(), Self::Error> {
        match update.leader {
            ValueDiff::None => {}
            ValueDiff::Replace(current_leader, new_leader) => {
                if current_leader != self.current_leader {
                    return Err(Error::UpdateHasInvalidCurrentLeader(
                        self.current_leader.clone(),
                        current_leader,
                    ));
                } else {
                    self.current_leader = new_leader;
                }
            }
        }
        Ok(())
    }

    #[inline]
    fn get_leader_at(
        &self,
        date: <Self::Block as property::Block>::Date,
    ) -> Result<Self::LeaderId, Self::Error> {
        let BftRoundRobinIndex(ofs) = self.offset(date.slot_id);
        Ok(self.leaders[ofs].clone())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::BlockHasInvalidLeader(expected, found) => write!(
                f,
                "Invalid block leader, expected {:?} but the given block was signed by {:?}",
                expected, found
            ),
            Error::BlockSignatureIsInvalid => write!(f, "The block signature is not valid"),
            Error::UpdateHasInvalidCurrentLeader(current, found) => write!(
                f,
                "Update has an incompatible leader, we expect to update from {:?} but we are at {:?}",
                found, current
            ),
        }
    }
}
impl std::error::Error for Error {}
