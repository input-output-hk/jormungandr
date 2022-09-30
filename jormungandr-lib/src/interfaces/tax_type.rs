use crate::interfaces::{Ratio, Value};
use chain_impl_mockchain::rewards;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Copy)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TaxType {
    pub fixed: Value,

    pub ratio: Ratio,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_limit: Option<NonZeroU64>,
}

/* ************** Conversion *********************************** */

impl From<TaxType> for rewards::TaxType {
    fn from(tax_type: TaxType) -> Self {
        rewards::TaxType {
            fixed: tax_type.fixed.into(),
            ratio: tax_type.ratio.into(),
            max_limit: tax_type.max_limit,
        }
    }
}

impl From<rewards::TaxType> for TaxType {
    fn from(tax_type: rewards::TaxType) -> Self {
        TaxType {
            fixed: tax_type.fixed.into(),
            ratio: tax_type.ratio.into(),
            max_limit: tax_type.max_limit,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use std::num::NonZeroU64;

    impl Arbitrary for TaxType {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            let denom = u64::arbitrary(g) + 1;
            let num = u64::arbitrary(g) % denom;

            let ratio = Ratio::new_checked(num, denom).unwrap();
            TaxType {
                fixed: Value::arbitrary(g),
                ratio,
                max_limit: NonZeroU64::new(Arbitrary::arbitrary(g)),
            }
        }
    }

    #[test]
    fn value_serde_yaml() {
        const FIXED: u64 = 8170;
        const NUMERATOR: u64 = 192;
        const DENOMINATOR: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(1291) };
        let tax_type = TaxType {
            fixed: FIXED.into(),
            ratio: Ratio::new(NUMERATOR, DENOMINATOR),
            max_limit: None,
        };

        assert_eq!(
            serde_yaml::to_string(&tax_type).unwrap(),
            format!(
                "---\nfixed: {}\nratio: {}/{}\n",
                FIXED, NUMERATOR, DENOMINATOR
            )
        );
    }

    #[test]
    fn value_serde_yaml_with_max_limit() {
        const FIXED: u64 = 8170;
        const NUMERATOR: u64 = 192;
        const DENOMINATOR: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(1291) };
        const MAX_LIMIT: u64 = 2028;
        let tax_type = TaxType {
            fixed: FIXED.into(),
            ratio: Ratio::new(NUMERATOR, DENOMINATOR),
            max_limit: NonZeroU64::new(MAX_LIMIT),
        };

        assert_eq!(
            serde_yaml::to_string(&tax_type).unwrap(),
            format!(
                "---\nfixed: {}\nratio: {}/{}\nmax_limit: {}\n",
                FIXED, NUMERATOR, DENOMINATOR, MAX_LIMIT
            )
        );
    }

    quickcheck! {
        fn value_serde_human_readable_encode_decode(value: TaxType) -> TestResult {
            let s = serde_yaml::to_string(&value).unwrap();
            let value_dec: TaxType = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(value_dec == value)
        }
    }
}
