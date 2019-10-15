use crate::header::{BlockVersion, ChainLength, HeaderId, Header};
use crate::date::BlockDate;
use chain_core::property;

impl property::ChainLength for ChainLength {
    fn next(&self) -> Self {
        self.increase()
    }
}

impl property::Header for Header {
    type Id = HeaderId;
    type Date = BlockDate;
    type Version = BlockVersion;
    type ChainLength = ChainLength;

    fn id(&self) -> Self::Id {
        self.hash()
    }
    fn parent_id(&self) -> Self::Id {
        self.block_parent_hash()
    }
    fn chain_length(&self) -> Self::ChainLength {
        self.chain_length()
    }
    fn date(&self) -> Self::Date {
        self.block_date()
    }
    fn version(&self) -> Self::Version {
        self.block_version()
    }
}

#[cfg(test)]
mod test {
    use crate::block::ConsensusVersion;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for ConsensusVersion {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            ConsensusVersion::from_u16(u16::arbitrary(g) % 2 + 1).unwrap()
        }
    }
}
