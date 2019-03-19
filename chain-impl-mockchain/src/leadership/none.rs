use crate::{
    block::{BlockDate, Header, Proof},
    leadership::{self, Error, ErrorKind, Verification},
};

/// Object for when there is no leadership for the block creation
///
/// This is a case that can happen when one is creating the `BlockZero`.
///
/// # Error
///
/// The NoLeadership mode may fail to produce a diff if the Block is not
/// a `NoLeadership` block
pub struct NoLeadership;

impl NoLeadership {
    pub(crate) fn verify(&self, block_header: &Header) -> Verification {
        match &block_header.proof() {
            Proof::None => Verification::Success,
            _ => Verification::Failure(Error::new(ErrorKind::InvalidLeaderSignature)),
        }
    }

    #[inline]
    pub(crate) fn get_leader_at(&self, _date: BlockDate) -> Result<leadership::LeaderId, Error> {
        Ok(leadership::LeaderId::None)
    }
}
