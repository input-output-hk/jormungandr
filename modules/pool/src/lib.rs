use chain_impl_mockchain::{
    block::BlockDate,
    fragment::{Fragment, FragmentId},
    ledger::{self, Ledger, LedgerParameters},
    transaction::{BalanceError, Transaction},
};
use thiserror::Error;

/// Implementation of a fragments pool. This pool performs two functions:
///
/// * Storage of pending fragments.
/// * Selections of fragments for a new block.
pub struct FragmentsPool {
    pool: lru::LruCache<FragmentId, Fragment>,
}

pub struct Configuration {
    /// The maximum number of entries this pool can keep as pending.
    pub max_pool_entries: usize,
}

pub struct SelectionResult {
    /// Fragments selected for a new block.
    pub selected: Vec<Fragment>,
    /// Information about fragments that could not be applied to the ledger.
    pub rejected: Vec<RejectedFragmentSelection>,
}

/// Information about a fragment that could not be applied to the ledger.
pub struct RejectedFragmentSelection {
    pub id: FragmentId,
    pub reason: ledger::Error,
}

/// Information about a fragment that could not be registered in the pool.
pub struct RejectedFragmentRegistration {
    pub id: FragmentId,
    pub reason: RegistrationError,
}

#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error(transparent)]
    Balance(#[from] BalanceError),
    #[error("the provided fragment must not be present outside of block0")]
    Block0Fragment,
    #[error("the provided fragment is currently unsupported")]
    UnsupportedFragment,
    #[error("tried to add a fragment already present in the pool")]
    KnownFragment,
}

impl FragmentsPool {
    pub fn new(config: Configuration) -> Self {
        let pool = lru::LruCache::new(config.max_pool_entries);
        Self { pool }
    }

    /// Register new fragments in this pool. If the total number of fragments
    /// exceeds `Configuration::max_pool_entries`, older pool entries will be
    /// removed.
    ///
    /// # Returns
    ///
    /// Information about fragments that cannot be added to the pool.
    pub fn register(&mut self, fragments: Vec<Fragment>) -> Vec<RejectedFragmentRegistration> {
        let mut rejected = Vec::new();

        for fragment in fragments {
            let hash = fragment.hash();
            if self.pool.peek(&hash).is_some() {
                rejected.push(RejectedFragmentRegistration {
                    id: hash,
                    reason: RegistrationError::KnownFragment,
                });
            }
            if let Err(reason) = verify_fragment(&fragment) {
                rejected.push(RejectedFragmentRegistration { id: hash, reason });
            }
            self.pool.put(hash, fragment);
        }

        rejected
    }

    /// Select fragments for a new block. All processed fragments (selected and
    /// rejected) are removed from the pool.
    ///
    /// # Arguments
    ///
    /// * `ledger` - current ledger state.
    /// * `ledger_params` - ledger parameters for the current epoch.
    /// * `block_date` - the date of the block that is being built.
    ///
    /// # Returns
    ///
    /// Fragments to be added to the new block and information the ones that
    /// were rejected.
    pub fn select(
        &mut self,
        mut ledger: Ledger,
        ledger_params: &LedgerParameters,
        block_date: BlockDate,
    ) -> SelectionResult {
        let fragments_number_max = ledger_params.block_content_max_size;

        let mut selected = Vec::new();
        let mut rejected = Vec::new();

        while let Some((hash, fragment)) = self.pool.pop_lru() {
            match ledger.apply_fragment(&ledger_params, &fragment, block_date) {
                Ok(new_ledger) => {
                    ledger = new_ledger;
                    selected.push(fragment);
                }
                Err(reason) => rejected.push(RejectedFragmentSelection { id: hash, reason }),
            }

            if selected.len() as u32 >= fragments_number_max {
                break;
            }
        }

        SelectionResult { selected, rejected }
    }

    /// Remove all fragments with the given ids.
    pub fn remove(&mut self, ids: Vec<FragmentId>) {
        for id in ids {
            self.pool.pop(&id);
        }
    }
}

fn verify_fragment(fragment: &Fragment) -> Result<(), RegistrationError> {
    match fragment {
        Fragment::Initial(_) => Err(RegistrationError::Block0Fragment),
        Fragment::OldUtxoDeclaration(_) => Err(RegistrationError::Block0Fragment),
        Fragment::Transaction(tx) => verify_transaction(tx),
        Fragment::OwnerStakeDelegation(tx) => verify_transaction(tx),
        Fragment::StakeDelegation(tx) => verify_transaction(tx),
        Fragment::PoolRegistration(tx) => verify_transaction(tx),
        Fragment::PoolRetirement(tx) => verify_transaction(tx),
        Fragment::PoolUpdate(tx) => verify_transaction(tx),
        Fragment::UpdateProposal(_) => Err(RegistrationError::UnsupportedFragment),
        Fragment::UpdateVote(_) => Err(RegistrationError::UnsupportedFragment),
        Fragment::VotePlan(tx) => verify_transaction(tx),
        Fragment::VoteCast(tx) => verify_transaction(tx),
        Fragment::VoteTally(tx) => verify_transaction(tx),
    }
}

fn verify_transaction<P>(tx: &Transaction<P>) -> Result<(), RegistrationError> {
    tx.verify_possibly_balanced().map_err(Into::into)
}
