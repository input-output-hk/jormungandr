use crate::interfaces::DEFAULT_BLOCK_CONTENT_MAX_SIZE;
use chain_impl_mockchain::fragment;
use serde::{Deserialize, Serialize};
use std::fmt;

/// the block content max size
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlockContentMaxSize(fragment::BlockContentSize);

impl fmt::Display for BlockContentMaxSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for BlockContentMaxSize {
    fn default() -> Self {
        BlockContentMaxSize(DEFAULT_BLOCK_CONTENT_MAX_SIZE)
    }
}

impl From<fragment::BlockContentSize> for BlockContentMaxSize {
    fn from(v: fragment::BlockContentSize) -> Self {
        BlockContentMaxSize(v)
    }
}

impl From<BlockContentMaxSize> for fragment::BlockContentSize {
    fn from(v: BlockContentMaxSize) -> Self {
        v.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for BlockContentMaxSize {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            BlockContentMaxSize(Arbitrary::arbitrary(g))
        }
    }
}
