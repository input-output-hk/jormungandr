use crate::blockcfg::{BlockDate, ChainLength, Header, HeaderHash};
use chain_impl_mockchain::multiverse::GCRoot;
use std::sync::Arc;

/// a reference to a block in the blockchain
#[derive(Clone)]
pub struct Ref {
    /// GCRoot holder for the object in the `Multiverse<Ledger>`.
    ///
    /// There is actually **one** pointer per block here as the ledger
    /// state changes with every block
    ledger_pointer: Arc<GCRoot>,

    /// GCRoot holder for the object in the `Multiverse<Leadership>`.
    ///
    /// This will allow us to retrieve quickly the Leader schedule
    /// for the on going epoch.
    leadership_pointer: Arc<GCRoot>,

    /// keep the Block header in memory, this will avoid retrieving
    /// the data from the storage if needs be
    header: Header,
}

impl Ref {
    /// create a new `Ref`
    pub fn new(ledger_pointer: GCRoot, leadership_pointer: GCRoot, header: Header) -> Self {
        #[cfg(debug_assertions)]
        use std::ops::Deref as _;

        debug_assert!(
            ledger_pointer.deref() == leadership_pointer.deref(),
            "expect both GCRoot to be the same"
        );
        debug_assert!(
            ledger_pointer.deref() == &header.hash(),
            "expect the GCRoot to be for the same `Header`"
        );

        Ref {
            ledger_pointer: Arc::new(ledger_pointer),
            leadership_pointer: Arc::new(leadership_pointer),
            header,
        }
    }

    /// retrieve the header hash of the `Ref`
    #[inline]
    pub fn hash(&self) -> &HeaderHash {
        use std::ops::Deref as _;

        self.ledger_pointer.deref()
    }

    /// access the reference's parent hash
    #[inline]
    pub fn block_parent_hash(&self) -> &HeaderHash {
        self.header().block_parent_hash()
    }

    /// retrieve the block date of the `Ref`
    #[inline]
    pub fn block_date(&self) -> &BlockDate {
        self.header().block_date()
    }

    /// retrieve the chain length, the number of blocks created
    /// between the block0 and this block. This is useful to compare
    /// the density of 2 branches.
    #[inline]
    pub fn chain_length(&self) -> ChainLength {
        self.header().chain_length()
    }

    /// access the `Header` of the block pointed by this `Ref`
    #[inline(always)]
    pub fn header(&self) -> &Header {
        &self.header
    }
}
