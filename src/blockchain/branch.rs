use crate::blockcfg::{ChainLength, HeaderHash};
use chain_impl_mockchain::multiverse::GCRoot;
use std::sync::Arc;

/// `Branch` or `Fork` are the different _propositions_ of what is
/// the current state of the ledger
///
/// In some modes, like in BFT, it is very unlikely (near impossible)
/// to experiences competitive branches because the leadership is
/// deterministic and *absolute*: only one node is authorized to create
/// a new block at a time.
///
/// In other modes, like in genesis praos, the leadership is deterministic
/// (in the sense we can reproduce the result in the same circumstances)
/// but it is not necessarily *absolute*: it is possible that multiple nodes
/// are elected to propose the next block.
///
/// This Branch structure is useful to maintain states of different branches
/// as well as the consensus branch: the **tip**.
#[derive(Clone)]
pub struct Branch {
    /// Make sure we hold the branch's details for as long as we need to
    /// in the multiverse.
    reference: Arc<GCRoot>,

    /// keep the chain length details of the branch
    ///
    /// This is a useful parameter to make choices regarding
    /// competitive branch (for the consensus, the choice of the **tip**).
    chain_length: ChainLength,
}

impl Branch {
    /// create a new branch from the given GCRoot
    #[inline]
    pub fn new(reference: GCRoot, chain_length: ChainLength) -> Self {
        Branch {
            reference: Arc::new(reference),
            chain_length,
        }
    }

    /// get the branch latest block hash
    #[inline]
    pub fn hash(&self) -> HeaderHash {
        **self.reference
    }

    /// get the branch latest block hash
    #[inline]
    pub fn chain_length(&self) -> &ChainLength {
        &self.chain_length
    }
}
