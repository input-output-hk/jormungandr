use chain_impl_mockchain::rewards;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, num::NonZeroU64, str::FromStr};
use thiserror::Error;

/// Ratio in the blockchain.
///
/// for example, used to represent the ratio of a setting in the stake pool
/// registration certificate.
///
#[derive(Debug, Clone, Copy)]
pub struct Ratio(rewards::Ratio);

impl Ratio {
    pub const fn new(numerator: u64, denominator: NonZeroU64) -> Self {
        Ratio(rewards::Ratio {
            numerator,
            denominator,
        })
    }

    pub fn new_checked(numerator: u64, denominator: u64) -> Option<Self> {
        NonZeroU64::new(denominator).map(move |denominator| Self::new(numerator, denominator))
    }
}

/* ---------------- Display ------------------------------------------------ */

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{numerator}/{denominator}",
            numerator = self.0.numerator,
            denominator = self.0.denominator
        )
    }
}

#[derive(Clone, Debug, Error)]
pub enum ParseRatioError {
    #[error("{source}")]
    InvalidInt {
        #[from]
        source: std::num::ParseIntError,
    },

    #[error("Missing numerator part of the Ratio")]
    MissingNumerator,

    #[error("Missing denominator part of the Ratio")]
    MissingDenominator,
}

impl FromStr for Ratio {
    type Err = ParseRatioError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split("/");

        let numerator = if let Some(numerator) = split.next() {
            numerator.parse::<u64>()?
        } else {
            return Err(ParseRatioError::MissingNumerator);
        };

        let denominator = if let Some(denominator) = split.next() {
            denominator.parse::<NonZeroU64>()?
        } else {
            return Err(ParseRatioError::MissingNumerator);
        };

        Ok(Ratio(rewards::Ratio {
            numerator,
            denominator,
        }))
    }
}

/* ---------------- Comparison ---------------------------------------------- */

impl PartialEq<Self> for Ratio {
    fn eq(&self, other: &Self) -> bool {
        self.0.numerator == other.0.numerator && self.0.denominator == other.0.denominator
    }
}

impl Eq for Ratio {}

/* ---------------- AsRef -------------------------------------------------- */

impl AsRef<rewards::Ratio> for Ratio {
    fn as_ref(&self) -> &rewards::Ratio {
        &self.0
    }
}

/* ---------------- Conversion --------------------------------------------- */

impl From<rewards::Ratio> for Ratio {
    fn from(v: rewards::Ratio) -> Self {
        Ratio(v)
    }
}

impl From<Ratio> for rewards::Ratio {
    fn from(v: Ratio) -> Self {
        v.0
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for Ratio {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Ratio {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;

        String::deserialize(deserializer)
            .map_err(D::Error::custom)
            .and_then(|s| s.parse().map_err(D::Error::custom))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use std::num::NonZeroU64;

    impl Arbitrary for Ratio {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            Ratio(rewards::Ratio {
                numerator: Arbitrary::arbitrary(g),
                denominator: NonZeroU64::new(Arbitrary::arbitrary(g))
                    .unwrap_or(NonZeroU64::new(1).unwrap()),
            })
        }
    }

    #[test]
    fn value_display_as_u64() {
        const NUMERATOR: u64 = 928170;
        const DENOMINATOR: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(1291) };
        let ratio = Ratio(rewards::Ratio {
            numerator: NUMERATOR,
            denominator: DENOMINATOR,
        });

        assert_eq!(ratio.to_string(), format!("{}/{}", NUMERATOR, DENOMINATOR))
    }

    #[test]
    fn value_serde_as_u64() {
        const NUMERATOR: u64 = 928170;
        const DENOMINATOR: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(1291) };
        let ratio = Ratio(rewards::Ratio {
            numerator: NUMERATOR,
            denominator: DENOMINATOR,
        });

        assert_eq!(
            serde_yaml::to_string(&ratio).unwrap(),
            format!("---\n{}/{}", NUMERATOR, DENOMINATOR)
        );
    }

    quickcheck! {
        fn value_display_parse(value: Ratio) -> TestResult {
            let s = value.to_string();
            let value_dec: Ratio = s.parse().unwrap();

            TestResult::from_bool(value_dec == value)
        }

        fn value_serde_human_readable_encode_decode(value: Ratio) -> TestResult {
            let s = serde_yaml::to_string(&value).unwrap();
            let value_dec: Ratio = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(value_dec == value)
        }

        fn value_serde_binary_encode_decode(value: Ratio) -> TestResult {
            let s = bincode::serialize(&value).unwrap();
            let value_dec: Ratio = bincode::deserialize(&s).unwrap();

            TestResult::from_bool(value_dec == value)
        }
    }
}
