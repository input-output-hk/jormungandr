use crate::{
    blockcfg::{ChainLength, HeaderHash, Ledger},
    blockchain::Branch,
};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Error that may happen if we cannot update the [`Tip`].
///
/// [`Tip`]: ./struct.tip.html
#[derive(Clone, Debug)]
pub struct TipReplaceError;

/// Error that may happen if we cannot access the [`Tip`].
///
/// [`Tip`]: ./struct.tip.html
#[derive(Clone, Debug)]
pub struct TipGetError;

/// `Tip` of the blockchain, can be safely shared between
/// different objects
///
/// This object is safe to clone, under the hood it is an `Arc<RwLock<...>>`
#[derive(Clone)]
pub struct Tip {
    branch: Arc<RwLock<Branch>>,
}
impl Tip {
    /// create a new tip
    #[inline]
    pub fn new(branch: Branch) -> Self {
        Tip {
            branch: Arc::new(RwLock::new(branch)),
        }
    }

    /// Update the Tip with the new given branch
    ///
    /// # Error
    ///
    /// This function might return an error if the underlying lock is
    /// poisoned.
    ///
    pub fn replace_with(&mut self, branch: Branch) -> Result<(), TipReplaceError> {
        *(self.branch.write()?) = branch;
        Ok(())
    }

    /// get the hash of the Tip
    ///
    /// # Error
    ///
    /// This function might return an error if the underlying lock is
    /// poisoned.
    ///
    #[inline]
    pub fn hash(&self) -> Result<HeaderHash, TipGetError> {
        Ok(self.branch.read()?.hash())
    }

    /// get the hash of the Tip
    ///
    /// # Error
    ///
    /// This function might return an error if the underlying lock is
    /// poisoned.
    ///
    #[inline]
    pub fn chain_length(&self) -> Result<ChainLength, TipGetError> {
        Ok(self.branch.read()?.chain_length().clone())
    }

    /// get the ledger of the Tip
    ///
    /// # Error
    ///
    /// This function might return an error if the underlying lock is
    /// poisoned.
    ///
    #[inline]
    pub fn ledger(&self) -> Result<Ledger, TipGetError> {
        Ok(self.branch.read()?.ledger().clone())
    }
}

impl std::fmt::Display for TipReplaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Cannot change the TIP's branch")
    }
}
impl std::fmt::Display for TipGetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Cannot access the TIP data...")
    }
}

impl std::error::Error for TipReplaceError {}
impl std::error::Error for TipGetError {}
impl<'a> From<PoisonError<RwLockReadGuard<'a, Branch>>> for TipGetError {
    fn from(_e: PoisonError<RwLockReadGuard<'a, Branch>>) -> Self {
        TipGetError
    }
}
impl<'a> From<PoisonError<RwLockWriteGuard<'a, Branch>>> for TipReplaceError {
    fn from(_e: PoisonError<RwLockWriteGuard<'a, Branch>>) -> Self {
        TipReplaceError
    }
}
