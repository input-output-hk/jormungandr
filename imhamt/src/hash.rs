pub use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

const SIZE: usize = 32;

/// Hash of a key
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HashedKey(pub u64);

impl HashedKey {
    /// Compute the HashedKey from a key 'k'
    pub fn compute<H: Hasher + Default, K: Hash>(_: PhantomData<H>, k: &K) -> Self {
        let mut hs = H::default();
        k.hash(&mut hs);
        HashedKey(hs.finish())
    }
}

impl HashedKey {
    /// get the index associated with a specific level
    #[inline]
    pub fn level_index(&self, level: usize) -> LevelIndex {
        // group of 5 bits
        let shift = level as u32 * 5;
        let idx = (self.0.wrapping_shr(shift) & 0b11111) as usize;
        assert!(idx < SIZE);
        LevelIndex(idx)
    }
}

/// Hash value subgroup per level used as Index for children
///
/// A number that is between 0 and the maximum number of children
/// elements in a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelIndex(pub usize);

impl LevelIndex {
    #[inline]
    pub fn mask(&self) -> u32 {
        1u32.wrapping_shl(self.0 as u32)
    }
}
