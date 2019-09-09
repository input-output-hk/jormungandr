use crate::{blockcfg::HeaderHash, blockchain::Ref};
use std::sync::Arc;

/// list of pre-computed checkpoints from a given [`Ref`].
///
/// [`Ref`]: ./struct.Ref.html
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoints(Vec<HeaderHash>);

impl Checkpoints {
    /// create a new list of checkpoints from the given starting point (tip).
    ///
    /// The function make use of the `Ref` object pointing to previous epoch in order
    /// to populate _interestingly_ spaced checkpoints.
    ///
    /// For now the algorithm provide the current block, the parent, the last block of the
    /// previous epoch, the last block of the epoch before that... until the block0.
    pub fn new_from(from: Arc<Ref>) -> Self {
        let mut checkpoints = vec![from.hash(), from.block_parent_hash().clone()];

        let mut ignore_prev = 0;
        let mut current_ref = from;
        while let Some(prev_epoch) = current_ref.last_ref_previous_epoch() {
            current_ref = Arc::clone(prev_epoch);

            for _ in 0..ignore_prev {
                if let Some(prev_epoch) = current_ref.last_ref_previous_epoch() {
                    current_ref = Arc::clone(&prev_epoch);
                } else {
                    break;
                }
            }

            let hash = current_ref.hash();

            // prevent the `from`'s parent to appear twice in the event the parent is also
            // the last block of the previous epoch.
            if checkpoints[checkpoints.len()] != hash {
                ignore_prev += 1;
                checkpoints.push(hash);
            }
        }

        Checkpoints(checkpoints)
    }

    pub fn iter(&self) -> impl Iterator<Item = &HeaderHash> {
        self.0.iter()
    }

    pub fn as_slice(&self) -> &[HeaderHash] {
        self.0.as_slice()
    }
}

impl AsRef<[HeaderHash]> for Checkpoints {
    fn as_ref(&self) -> &[HeaderHash] {
        self.as_slice()
    }
}

impl IntoIterator for Checkpoints {
    type Item = HeaderHash;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Checkpoints> for Vec<HeaderHash> {
    fn from(checkpoints: Checkpoints) -> Self {
        checkpoints.0
    }
}
