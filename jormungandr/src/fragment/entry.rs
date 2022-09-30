use crate::{
    blockcfg::{Value, ValueError},
    fragment::{Fragment, FragmentId},
};
use chain_core::property::Serialize;
use std::time::SystemTime;

pub struct PoolEntry {
    // reference of the fragment stored in the pool
    fragment_ref: FragmentId,
    /// fee of the fragment, does not include the fee of
    /// descendants entries or ancestors
    fragment_fee: Value,
    /// size of the fragment in the memory pool
    fragment_size: usize,
    /// time when the entry was added to the pool
    received_at: SystemTime,
    /// the fee of the accumulated descendant fragments
    /// does not include the fee of this entry
    descendants_fee: Value,
    /// the size of the accumulated descendant fragments
    /// Does not include the size of this entry
    descendants_size: usize,
    /// the fee of the accumulated ancestor fragments
    /// does not include the fee of this entry
    ancestors_fee: Value,
    /// the size of the accumulated ancestor fragments
    /// Does not include the size of this entry
    ancestors_size: usize,
}

impl PoolEntry {
    pub fn new(fragment: &Fragment) -> Self {
        let fragment_size = fragment.serialized_size();
        let fragment_ref = fragment.hash();
        // TODO: the fragment fee is not yet computed. Yet we should
        // have an explicit fee in the message. So we need to be able
        // to extract this information without the need to compute the
        // fee from the ledger's fee settings.
        let fragment_fee = Value::zero();

        PoolEntry {
            fragment_ref,
            fragment_fee,
            fragment_size,
            received_at: SystemTime::now(),

            // when this entry is added in the pool, it has no
            // descendant
            descendants_fee: Value::zero(),
            descendants_size: 0usize,

            // when this entry is added to the pool, we need to know
            // about the different entries in order to compute the following:
            ancestors_fee: Value::zero(),
            ancestors_size: 0usize,
        }
    }

    #[inline]
    pub fn fragment_ref(&self) -> &FragmentId {
        &self.fragment_ref
    }
    #[inline]
    pub fn fragment_fee(&self) -> &Value {
        &self.fragment_fee
    }
    #[inline]
    pub fn fragment_size(&self) -> &usize {
        &self.fragment_size
    }
    #[inline]
    pub fn received_at(&self) -> &SystemTime {
        &self.received_at
    }
    #[inline]
    pub fn with_descendants_fee(&self) -> Result<Value, ValueError> {
        self.descendants_fee + self.fragment_fee
    }
    #[inline]
    pub fn with_descendants_size(&self) -> usize {
        self.descendants_size + self.fragment_size
    }
    #[inline]
    pub fn with_ancestors_fee(&self) -> Result<Value, ValueError> {
        self.ancestors_fee + self.fragment_fee
    }
    #[inline]
    pub fn with_ancestors_size(&self) -> usize {
        self.ancestors_size + self.fragment_size
    }
}
