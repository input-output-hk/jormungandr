use crate::interfaces::{Ratio, Value};
use chain_impl_mockchain::rewards;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::num::NonZeroU64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RewardLimitByStake {
    pub numerator: u32,
    pub denominator: NonZeroU32,
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use std::num::NonZeroU64;

    impl Arbitrary for RewardLimitByStake {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            RewardLimitByStake {
                numerator: Arbitrary::arbitrary(g),
                denominator: NonZeroU32::new(Arbitrary::arbitrary(g))
                    .unwrap_or(NonZeroU32::new(1).unwrap()),
            }
        }
    }

    #[test]
    fn value_serde_yaml() {
        const NUMERATOR: u32 = 15;
        const DENOMINATOR: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(100) };
        let reward_limit = RewardLimitByStake {
            numerator: NUMERATOR,
            denominator: DENOMINATOR,
        };

        assert_eq!(
            serde_yaml::to_string(&reward_limit).unwrap(),
            format!(
                "---\nnumerator: {}\ndenominator: {}",
                NUMERATOR, DENOMINATOR
            )
        );
    }
}

/* ************** Conversion *********************************** */

impl From<RewardLimitByStake> for rewards::RewardLimitByStake {
    fn from(rewards_limit: RewardLimitByStake) -> Self {
        rewards::RewardLimitByStake {
            numerator: rewards_limit.numerator,
            denominator: rewards_limit.denominator.into(),
        }
    }
}

impl From<rewards::RewardLimitByStake> for RewardLimitByStake {
    fn from(rewards_limit: rewards::RewardLimitByStake) -> Self {
        RewardLimitByStake {
            numerator: rewards_limit.numerator,
            denominator: rewards_limit.denominator.into(),
        }
    }
}
