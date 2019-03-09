use crate::{
    block::{Block, BlockVersion, Proof, BLOCK_VERSION_CONSENSUS_NONE},
    leadership::{Error, ErrorKind, PublicLeader, Update},
};
use chain_core::property::{self, Block as _, LeaderSelection};

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
    type Update = Update;
    type Error = Error;

    fn diff(&self, input: &Self::Block) -> Result<Self::Update, Self::Error> {
        if input.version() != BLOCK_VERSION_CONSENSUS_NONE {
            return Err(Error {
                kind: ErrorKind::IncompatibleBlockVersion,
                cause: Some(Box::new(NoLeadershipError::IncompatibleBlockVersion)),
            });
        }
        match &input.header.proof {
            Proof::None => {
                let mut update = <Self::Update as property::Update>::empty();
                update.previous_leader = PublicLeader::None;
                update.next_leader = PublicLeader::None;
                Ok(update)
            }
            Proof::Bft(_) => Err(Error {
                kind: ErrorKind::IncompatibleLeadershipMode,
                cause: Some(Box::new(NoLeadershipError::BlockProofIsDifferent)),
            }),
            Proof::GenesisPraos(_) => Err(Error {
                kind: ErrorKind::IncompatibleLeadershipMode,
                cause: Some(Box::new(NoLeadershipError::BlockProofIsDifferent)),
            }),
        }
    }

    fn apply(&mut self, _update: Self::Update) -> Result<(), Self::Error> {
        Ok(())
    }

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
