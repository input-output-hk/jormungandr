use super::DEFAULT_PROPOSAL_EXPIRATION;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProposalExpiration(u32);

impl fmt::Display for ProposalExpiration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for ProposalExpiration {
    fn default() -> Self {
        ProposalExpiration(DEFAULT_PROPOSAL_EXPIRATION)
    }
}

impl From<u32> for ProposalExpiration {
    fn from(v: u32) -> Self {
        ProposalExpiration(v)
    }
}

impl From<ProposalExpiration> for u32 {
    fn from(v: ProposalExpiration) -> Self {
        v.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for ProposalExpiration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            ProposalExpiration(Arbitrary::arbitrary(g))
        }
    }
}
