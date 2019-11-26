use super::components::ChainLength;
use crate::certificate::PoolId;
use crate::date::BlockDate;
use crate::leadership;

/// Genesis Praos related data extract from the header
#[derive(Debug, Clone)]
pub(crate) struct HeaderGPContentEvalContext {
    pub(crate) nonce: leadership::genesis::Nonce,
    pub(crate) pool_creator: PoolId,
}

/// This is the data extracted from a header related to content evaluation
#[derive(Debug, Clone)]
pub struct HeaderContentEvalContext {
    pub(crate) block_date: BlockDate,
    pub(crate) chain_length: ChainLength,
    pub(crate) gp_content: Option<HeaderGPContentEvalContext>,
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for HeaderGPContentEvalContext {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            HeaderGPContentEvalContext {
                nonce: Arbitrary::arbitrary(g),
                pool_creator: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for HeaderContentEvalContext {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            HeaderContentEvalContext {
                block_date: Arbitrary::arbitrary(g),
                chain_length: Arbitrary::arbitrary(g),
                gp_content: Arbitrary::arbitrary(g),
            }
        }
    }
}
