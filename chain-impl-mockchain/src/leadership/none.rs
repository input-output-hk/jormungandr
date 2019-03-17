use crate::{
    block::Block,
    leadership::{self, Error},
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

impl LeaderSelection for NoLeadership {
    type LeaderId = leadership::LeaderId;
    type Block = Block;
    type Error = Error;

    fn get_leader_at(
        &self,
        _date: <Self::Block as property::Block>::Date,
    ) -> Result<Self::LeaderId, Self::Error> {
        Ok(leadership::LeaderId::None)
    }
}
