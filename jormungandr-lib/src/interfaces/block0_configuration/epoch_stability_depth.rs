use crate::interfaces::DEFAULT_EPOCH_STABILITY_DEPTH;
use serde::{Deserialize, Serialize};
use std::fmt;

/// epoch stability depth
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EpochStabilityDepth(pub(crate) u32);

impl fmt::Display for EpochStabilityDepth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for EpochStabilityDepth {
    fn default() -> Self {
        EpochStabilityDepth(DEFAULT_EPOCH_STABILITY_DEPTH)
    }
}

impl From<u32> for EpochStabilityDepth {
    fn from(v: u32) -> Self {
        EpochStabilityDepth(v)
    }
}

impl From<EpochStabilityDepth> for u32 {
    fn from(v: EpochStabilityDepth) -> Self {
        v.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for EpochStabilityDepth {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            EpochStabilityDepth(Arbitrary::arbitrary(g))
        }
    }
}
