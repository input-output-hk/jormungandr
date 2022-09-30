use crate::interfaces::Ratio;
use chain_impl_mockchain::{block::Epoch, config::RewardParams as RewardParamsStd};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Copy)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum RewardParams {
    Linear {
        constant: u64,
        ratio: Ratio,
        epoch_start: Epoch,
        epoch_rate: NonZeroU32,
    },
    Halving {
        constant: u64,
        ratio: Ratio,
        epoch_start: Epoch,
        epoch_rate: NonZeroU32,
    },
}

/* ************** Conversion *********************************** */

impl From<RewardParams> for RewardParamsStd {
    fn from(rp: RewardParams) -> Self {
        match rp {
            RewardParams::Linear {
                constant,
                ratio,
                epoch_start,
                epoch_rate,
            } => RewardParamsStd::Linear {
                constant,
                ratio: ratio.into(),
                epoch_start,
                epoch_rate,
            },
            RewardParams::Halving {
                constant,
                ratio,
                epoch_start,
                epoch_rate,
            } => RewardParamsStd::Halving {
                constant,
                ratio: ratio.into(),
                epoch_start,
                epoch_rate,
            },
        }
    }
}

impl From<RewardParamsStd> for RewardParams {
    fn from(rp: RewardParamsStd) -> Self {
        match rp {
            RewardParamsStd::Linear {
                constant,
                ratio,
                epoch_start,
                epoch_rate,
            } => RewardParams::Linear {
                constant,
                ratio: ratio.into(),
                epoch_start,
                epoch_rate,
            },
            RewardParamsStd::Halving {
                constant,
                ratio,
                epoch_start,
                epoch_rate,
            } => RewardParams::Halving {
                constant,
                ratio: ratio.into(),
                epoch_start,
                epoch_rate,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use std::num::NonZeroU64;

    impl Arbitrary for RewardParams {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            if bool::arbitrary(g) {
                Self::Linear {
                    constant: u64::arbitrary(g),
                    ratio: Ratio::arbitrary(g),
                    epoch_start: Epoch::arbitrary(g),
                    epoch_rate: NonZeroU32::new(Arbitrary::arbitrary(g))
                        .unwrap_or_else(|| NonZeroU32::new(1).unwrap()),
                }
            } else {
                Self::Halving {
                    constant: u64::arbitrary(g),
                    ratio: Ratio::arbitrary(g),
                    epoch_start: Epoch::arbitrary(g),
                    epoch_rate: NonZeroU32::new(Arbitrary::arbitrary(g))
                        .unwrap_or_else(|| NonZeroU32::new(1).unwrap()),
                }
            }
        }
    }

    #[test]
    fn linear_serde_yaml() {
        const CONSTANT: u64 = 8170;
        const RATIO_NUMERATOR: u64 = 13;
        const RATIO_DENOMINATOR: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(19) };
        const EPOCH_START: Epoch = 2;
        const EPOCH_RATE: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(5) };

        let parameters = RewardParams::Linear {
            constant: CONSTANT,
            ratio: Ratio::new(RATIO_NUMERATOR, RATIO_DENOMINATOR),
            epoch_start: EPOCH_START,
            epoch_rate: EPOCH_RATE,
        };

        assert_eq!(
            serde_yaml::to_string(&parameters).unwrap(),
            format!(
                "---\nlinear:\n  constant: {}\n  ratio: {}/{}\n  epoch_start: {}\n  epoch_rate: {}\n",
                CONSTANT, RATIO_NUMERATOR, RATIO_DENOMINATOR, EPOCH_START, EPOCH_RATE,
            )
        );
    }

    #[test]
    fn halving_serde_yaml() {
        const CONSTANT: u64 = 8170;
        const RATIO_NUMERATOR: u64 = 13;
        const RATIO_DENOMINATOR: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(19) };
        const EPOCH_START: Epoch = 2;
        const EPOCH_RATE: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(5) };

        let parameters = RewardParams::Halving {
            constant: CONSTANT,
            ratio: Ratio::new(RATIO_NUMERATOR, RATIO_DENOMINATOR),
            epoch_start: EPOCH_START,
            epoch_rate: EPOCH_RATE,
        };

        assert_eq!(
            serde_yaml::to_string(&parameters).unwrap(),
            format!(
                "---\nhalving:\n  constant: {}\n  ratio: {}/{}\n  epoch_start: {}\n  epoch_rate: {}\n",
                CONSTANT, RATIO_NUMERATOR, RATIO_DENOMINATOR, EPOCH_START, EPOCH_RATE,
            )
        );
    }

    quickcheck! {
        fn value_serde_human_readable_encode_decode(value: RewardParams) -> TestResult {
            let s = serde_yaml::to_string(&value).unwrap();
            let value_dec: RewardParams = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(value_dec == value)
        }
    }
}
