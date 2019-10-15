use crate::date::BlockDate;
use crate::leadership;
use super::components::ChainLength;

/// This is the data extracted from a header related to content evaluation
#[derive(Debug, Clone)]
pub struct HeaderContentEvalContext {
    pub block_date: BlockDate,
    pub chain_length: ChainLength,
    pub nonce: Option<leadership::genesis::Nonce>,
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for HeaderContentEvalContext {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            HeaderContentEvalContext {
                block_date: Arbitrary::arbitrary(g),
                chain_length: Arbitrary::arbitrary(g),
                nonce: Arbitrary::arbitrary(g),
            }
        }
    }
}
