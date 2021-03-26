use chain_impl_mockchain::stake;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

/// Stake in the blockchain, always printed as absolute Lovelace
///
/// Stake has some property to be human readable on standard display
///
/// ```
/// # use jormungandr_lib::interfaces::Stake;
/// # use chain_impl_mockchain::stake::Stake as StdStake;
///
/// let stake: Stake = StdStake(64).into();
///
/// println!("stake: {}", stake);
///
/// # assert_eq!(stake.to_string(), "64");
/// ```
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stake(stake::Stake);

/* ---------------- Display ------------------------------------------------ */

impl fmt::Display for Stake {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Stake {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(|v| Stake(stake::Stake(v)))
    }
}

/* ---------------- AsRef -------------------------------------------------- */

impl AsRef<stake::Stake> for Stake {
    fn as_ref(&self) -> &stake::Stake {
        &self.0
    }
}
/* ---------------- Conversion --------------------------------------------- */

impl From<stake::Stake> for Stake {
    fn from(v: stake::Stake) -> Self {
        Stake(v)
    }
}

impl From<Stake> for stake::Stake {
    fn from(v: Stake) -> Self {
        v.0
    }
}

impl From<u64> for Stake {
    fn from(v: u64) -> Self {
        Stake(stake::Stake(v))
    }
}

impl From<Stake> for u64 {
    fn from(stake: Stake) -> u64 {
        (stake.0).0
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for Stake {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_ref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Stake {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = u64::deserialize(deserializer)?;
        Ok(Stake(stake::Stake(v)))
    }
}

#[derive(Deserialize, Serialize)]
#[serde(transparent, remote = "stake::Stake")]
pub struct StakeDef(u64);

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for Stake {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            Stake(stake::Stake(u64::arbitrary(g)))
        }
    }

    #[test]
    fn stake_display_as_u64() {
        const STAKE: u64 = 928_170;
        let stake = Stake(stake::Stake(STAKE));

        assert_eq!(stake.to_string(), STAKE.to_string());
    }

    #[test]
    fn stake_serde_as_u64() {
        const STAKE: u64 = 928_170;
        let stake = Stake(stake::Stake(STAKE));

        assert_eq!(
            serde_yaml::to_string(&stake).unwrap(),
            format!("---\n{}\n", STAKE)
        );
    }

    quickcheck! {
        fn stake_display_parse(stake: Stake) -> TestResult {
            let s = stake.to_string();
            let stake_dec: Stake = s.parse().unwrap();

            TestResult::from_bool(stake_dec == stake)
        }

        fn stake_serde_human_readable_encode_decode(stake: Stake) -> TestResult {
            let s = serde_yaml::to_string(&stake).unwrap();
            let stake_dec: Stake = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(stake_dec == stake)
        }

        fn stake_serde_binary_encode_decode(stake: Stake) -> TestResult {
            let s = bincode::serialize(&stake).unwrap();
            let stake_dec: Stake = bincode::deserialize(&s).unwrap();

            TestResult::from_bool(stake_dec == stake)
        }
    }
}
