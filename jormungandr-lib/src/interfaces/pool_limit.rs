use crate::interfaces::{Ratio, Value};
use chain_impl_mockchain::rewards;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::num::NonZeroU64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PoolLimit {
    pub npools: NonZeroU32,
    pub npools_threshold: NonZeroU32,
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use std::num::NonZeroU64;
    impl Arbitrary for PoolLimit {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            PoolLimit {
                npools: NonZeroU32::new(Arbitrary::arbitrary(g))
                    .unwrap_or(NonZeroU32::new(1).unwrap()),
                npools_threshold: NonZeroU32::new(Arbitrary::arbitrary(g))
                    .unwrap_or(NonZeroU32::new(1).unwrap()),
            }
        }
    }

    #[test]
    fn value_serde_yaml() {
        const NPOOLS: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(123) };
        const NPOOLS_THRESHOLD: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(456) };
        let pool_limit = PoolLimit {
            npools: NPOOLS,
            npools_threshold: NPOOLS_THRESHOLD,
        };

        assert_eq!(
            serde_yaml::to_string(&pool_limit).unwrap(),
            format!(
                "---\nnpools: {}\nnpools_threshold: {}",
                NPOOLS, NPOOLS_THRESHOLD
            )
        );
    }
}

/* ************** Conversion *********************************** */

impl From<PoolLimit> for rewards::PoolLimit {
    fn from(rewards_limit: PoolLimit) -> Self {
        rewards::PoolLimit {
            npools: rewards_limit.npools.into(),
            npools_threshold: rewards_limit.npools_threshold.into(),
        }
    }
}

impl From<rewards::PoolLimit> for PoolLimit {
    fn from(rewards_limit: rewards::PoolLimit) -> Self {
        PoolLimit {
            npools: rewards_limit.npools.into(),
            npools_threshold: rewards_limit.npools_threshold.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RewardLimitByStake {
    pub numerator: u32,
    pub denominator: NonZeroU32,
}
