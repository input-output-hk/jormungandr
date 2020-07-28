use crate::Reference;
use chain_impl_mockchain::header::HeaderId;
use std::ops::Deref;

/// different checkpoints in time of the blockchain
///
/// This is useful to build a list of header ids that can be used to hint
/// the network or the storage about other blocks relevant for streaming
/// blocks.
///
/// the list is ordered from most recent to most ancient. It starts with
/// the given reference and its parent hash, then the hash of the previous epoch
/// then iterate epoch after epoch. Leaving increasingly larger holes between epoch
///
/// `hash -> parent hash -> epoch N -> N-1 -> N-3 -> N-7 -> N-15 -> N-31 ...`
#[derive(Debug, PartialEq, Eq)]
pub struct Checkpoints(Box<[HeaderId]>);

impl Checkpoints {
    pub fn new(from: &Reference) -> Self {
        let mut checkpoints = vec![from.hash(), from.block_parent_hash()];
        let mut cursor = from;

        let mut to_skip = 0;
        let mut skipped = 0;
        while let Some(prev) = cursor.previous_epoch_state() {
            cursor = prev.as_ref();
            if cursor.hash() == from.block_parent_hash() {
                // ignore the case where the block's parent is also the last block
                // of the previous epoch
                continue;
            }

            if skipped >= to_skip {
                checkpoints.push(cursor.hash());
                to_skip = 1 + to_skip * 2;
                skipped = 0;
            } else {
                skipped += 1;
            }
        }

        Self(checkpoints.into())
    }

    pub fn iter(&self) -> ::std::slice::Iter<HeaderId> {
        self.0.iter()
    }
}

impl AsRef<[HeaderId]> for Checkpoints {
    fn as_ref(&self) -> &[HeaderId] {
        self.0.as_ref()
    }
}

impl Deref for Checkpoints {
    type Target = [HeaderId];
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a> IntoIterator for &'a Checkpoints {
    type Item = &'a HeaderId;
    type IntoIter = ::std::slice::Iter<'a, HeaderId>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
