use crate::key::{PrivateKey, PublicKey};
use crate::transaction::Value;
use chain_core::property;
use std::collections::{HashMap, HashSet};

// For each stake pool, the total stake value, and the value for the
// stake pool members.
pub type StakeDistribution = HashMap<StakePoolId, (Value, HashMap<PublicKey, Value>)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeKeyInfo {
    /// Current stake pool this key is a member of, if any.
    pub pool: Option<StakePoolId>,
    // - reward account
    // - registration deposit (if variable)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolInfo {
    //owners: HashSet<PublicKey>,
    pub members: HashSet<PublicKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StakePoolId(pub PublicKey);

impl From<&PrivateKey> for StakePoolId {
    fn from(key: &PrivateKey) -> Self {
        StakePoolId(key.public())
    }
}

impl property::Serialize for StakePoolId {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.0.serialize(writer)
    }
}

impl property::Deserialize for StakePoolId {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        Ok(StakePoolId(PublicKey::deserialize(reader)?))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for StakePoolId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakePoolId(Arbitrary::arbitrary(g))
        }
    }
}
