use crate::{
    block::Block,
    leadership::{Error, PublicLeader},
};
use chain_core::property::{self, LeaderSelection};

/// Object for when there is no leadership for the block creation
///
/// This is a case that can happen when one is creating the `BlockZero`.
///
/// # Error
///
/// The NoLeadership mode may fail to produce a diff if the Block is not
/// a `NoLeadership` block
pub struct NoLeadership;

/// error that may happen during the NoLeadership mod
#[derive(Debug)]
pub enum NoLeadershipError {
    IncompatibleBlockVersion,
    BlockProofIsDifferent,
}

impl LeaderSelection for NoLeadership {
    type LeaderId = PublicLeader;
    type Block = Block;
    type Error = Error;

    fn get_leader_at(
        &self,
        _date: <Self::Block as property::Block>::Date,
    ) -> Result<Self::LeaderId, Self::Error> {
        Ok(PublicLeader::None)
    }
}

impl std::fmt::Display for NoLeadershipError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NoLeadershipError::IncompatibleBlockVersion => {
                write!(f, "The block version does no allow a NoLeadership mod")
            }
            NoLeadershipError::BlockProofIsDifferent => write!(
                f,
                "The block proof is different from the expected NoLeadership mod"
            ),
        }
    }
}

impl std::error::Error for NoLeadershipError {}
