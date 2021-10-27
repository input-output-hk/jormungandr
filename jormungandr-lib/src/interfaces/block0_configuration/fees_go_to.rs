use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

/// the settings for the fees to be redistributed to
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeesGoTo {
    /// the fees will be added to the epoch's reward pot to then be distributed amongst
    /// the pools.
    Rewards,
    /// the pools don't receive any rewards to add transactions in the blocks
    /// it is instead given entirely to the treasury.
    Treasury,
}

/* Display ****************************************************************** */

impl fmt::Display for FeesGoTo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Rewards => "rewards".fmt(f),
            Self::Treasury => "treasury".fmt(f),
        }
    }
}

#[derive(Debug, Error)]
#[error("Invalid fees go to setting. Expect \"rewards\" or \"treasury\" ")]
pub struct FromStrFeesGoToError;

impl FromStr for FeesGoTo {
    type Err = FromStrFeesGoToError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rewards" => Ok(Self::Rewards),
            "treasury" => Ok(Self::Treasury),
            _ => Err(FromStrFeesGoToError),
        }
    }
}

/* Conversion *************************************************************** */

impl From<bool> for FeesGoTo {
    fn from(fees_in_treasury: bool) -> Self {
        if fees_in_treasury {
            Self::Treasury
        } else {
            Self::Rewards
        }
    }
}

impl From<FeesGoTo> for bool {
    fn from(fees_in_treasury: FeesGoTo) -> Self {
        match fees_in_treasury {
            FeesGoTo::Treasury => true,
            FeesGoTo::Rewards => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for FeesGoTo {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            if bool::arbitrary(g) {
                Self::Rewards
            } else {
                Self::Treasury
            }
        }
    }

    quickcheck! {
        fn serde_encode_decode(fees_go_to: FeesGoTo) -> bool {
            let s = serde_yaml::to_string(&fees_go_to).unwrap();
            let fees_go_to_dec: FeesGoTo = serde_yaml::from_str(&s).unwrap();

            fees_go_to == fees_go_to_dec
        }

        fn display_from_str(fees_go_to: FeesGoTo) -> bool {
            let s = fees_go_to.to_string();
            let fees_go_to_dec: FeesGoTo = s.parse().unwrap();

            fees_go_to == fees_go_to_dec
        }
    }
}
