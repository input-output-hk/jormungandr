use crate::stake::StakePoolId;
use chain_core::{
    mempack::{ReadBuf, ReadError, Readable},
    property,
};

/// A self delegation to a specific StakePoolId.
///
/// This structure is not sufficient to identify the owner, and instead we rely on a special
/// authenticated transaction, which has 1 input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerStakeDelegation {
    pub stake_pool: StakePoolId,
}

impl property::Serialize for OwnerStakeDelegation {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.stake_pool.serialize(writer)
    }
}

impl Readable for OwnerStakeDelegation {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(Self {
            stake_pool: StakePoolId::read(buf)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for OwnerStakeDelegation {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                stake_pool: Arbitrary::arbitrary(g),
            }
        }
    }
}
