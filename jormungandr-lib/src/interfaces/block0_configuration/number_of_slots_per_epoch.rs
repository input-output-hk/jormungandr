use chain_impl_mockchain::config::ConfigParam;
use serde::{Deserialize, Deserializer, Serialize};
use std::{convert::TryFrom, fmt};

const DEFAULT_NUMBER_OF_SLOTS_PER_EPOCH: u32 = 720;
const MINIMUM_NUMBER_OF_SLOTS_PER_EPOCH: u32 = 1;
const MAXIMUM_NUMBER_OF_SLOTS_PER_EPOCH: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct NumberOfSlotsPerEpoch(pub(crate) u32);

impl NumberOfSlotsPerEpoch {
    /// minimal value for the number of slots per epoch
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::NumberOfSlotsPerEpoch;
    ///
    /// assert_eq!(NumberOfSlotsPerEpoch::MINIMUM, NumberOfSlotsPerEpoch::new(1).unwrap())
    /// ```
    pub const MINIMUM: Self = NumberOfSlotsPerEpoch(MINIMUM_NUMBER_OF_SLOTS_PER_EPOCH);

    /// maximal value for the number of slots per epoch
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::NumberOfSlotsPerEpoch;
    ///
    /// assert_eq!(NumberOfSlotsPerEpoch::MAXIMUM, NumberOfSlotsPerEpoch::new(1_000_000).unwrap())
    /// ```
    pub const MAXIMUM: Self = NumberOfSlotsPerEpoch(MAXIMUM_NUMBER_OF_SLOTS_PER_EPOCH);

    /// create a new `NumberOfSlotsPerEpoch` value
    ///
    /// returns `None` if the value is not within the boundaries of
    /// `NumberOfSlotsPerEpoch::MINIMUM` and `NumberOfSlotsPerEpoch::MAXIMUM`.
    pub fn new(v: u32) -> Option<Self> {
        if v < MINIMUM_NUMBER_OF_SLOTS_PER_EPOCH || MAXIMUM_NUMBER_OF_SLOTS_PER_EPOCH < v {
            None
        } else {
            Some(NumberOfSlotsPerEpoch(v))
        }
    }
}

custom_error! { pub TryFromNumberOfSlotsPerEpochError
    Incompatible = "Incompatible Config param, expected number of slots per epoch",
    Invalid { slots: u32 } = "invalid number of slots per epoch {slots}"
}

impl TryFrom<ConfigParam> for NumberOfSlotsPerEpoch {
    type Error = TryFromNumberOfSlotsPerEpochError;
    fn try_from(config_param: ConfigParam) -> Result<Self, Self::Error> {
        match config_param {
            ConfigParam::SlotsPerEpoch(slots) => NumberOfSlotsPerEpoch::new(slots)
                .ok_or(TryFromNumberOfSlotsPerEpochError::Invalid { slots }),
            _ => Err(TryFromNumberOfSlotsPerEpochError::Incompatible),
        }
    }
}

impl From<NumberOfSlotsPerEpoch> for ConfigParam {
    fn from(number_of_slots_per_epoch: NumberOfSlotsPerEpoch) -> Self {
        ConfigParam::SlotsPerEpoch(number_of_slots_per_epoch.0)
    }
}

impl fmt::Display for NumberOfSlotsPerEpoch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for NumberOfSlotsPerEpoch {
    fn default() -> Self {
        NumberOfSlotsPerEpoch::new(DEFAULT_NUMBER_OF_SLOTS_PER_EPOCH)
            .expect("Default should be a valid value at all time")
    }
}

impl<'de> Deserialize<'de> for NumberOfSlotsPerEpoch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        struct NumberOfSlotsPerEpochVisitor;
        impl<'de> Visitor<'de> for NumberOfSlotsPerEpochVisitor {
            type Value = NumberOfSlotsPerEpoch;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "Number of slots per epoch (between {} and {})",
                    NumberOfSlotsPerEpoch::MINIMUM,
                    NumberOfSlotsPerEpoch::MAXIMUM,
                )
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v < (MINIMUM_NUMBER_OF_SLOTS_PER_EPOCH as u64) {
                    Err(E::custom(format!(
                        "cannot have less than {} slots in an epoch",
                        NumberOfSlotsPerEpoch::MINIMUM,
                    )))
                } else if v > (MAXIMUM_NUMBER_OF_SLOTS_PER_EPOCH as u64) {
                    Err(E::custom(format!(
                        "cannot have more than {} slots in an epoch",
                        NumberOfSlotsPerEpoch::MAXIMUM,
                    )))
                } else {
                    Ok(NumberOfSlotsPerEpoch(v as u32))
                }
            }
        }
        deserializer.deserialize_u64(NumberOfSlotsPerEpochVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for NumberOfSlotsPerEpoch {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand::Rng as _;
            let v = g.gen_range(
                MINIMUM_NUMBER_OF_SLOTS_PER_EPOCH,
                MAXIMUM_NUMBER_OF_SLOTS_PER_EPOCH,
            );
            NumberOfSlotsPerEpoch(v)
        }
    }

    #[test]
    #[should_panic]
    fn deserialize_from_invalid_type() {
        const EXAMPLE: &'static str = "---\n\"928\"";

        let _: NumberOfSlotsPerEpoch = serde_yaml::from_str(EXAMPLE).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_from_below_bounds() {
        const VALUE: u32 = MINIMUM_NUMBER_OF_SLOTS_PER_EPOCH - 1;
        let example = format!("---\n{}", VALUE);

        let _: NumberOfSlotsPerEpoch = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_from_above_bounds() {
        const VALUE: u64 = (MAXIMUM_NUMBER_OF_SLOTS_PER_EPOCH as u64) + 1;
        let example = format!("---\n{}", VALUE);

        let _: NumberOfSlotsPerEpoch = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    fn deserialize_from_number() {
        const VALUE: u32 = 40;
        let example = format!("---\n{}", VALUE);

        let decoded: NumberOfSlotsPerEpoch = serde_yaml::from_str(&example).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    quickcheck! {
        fn serde_encode_decode(number_of_slots_per_epoch: NumberOfSlotsPerEpoch) -> bool {
            let s = serde_yaml::to_string(&number_of_slots_per_epoch).unwrap();
            let number_of_slots_per_epoch_dec: NumberOfSlotsPerEpoch = serde_yaml::from_str(&s).unwrap();

            number_of_slots_per_epoch == number_of_slots_per_epoch_dec
        }

        fn convert_from_to_config_param(number_of_slots_per_epoch: NumberOfSlotsPerEpoch) -> bool {
            let cp = ConfigParam::from(number_of_slots_per_epoch);
            let number_of_slots_per_epoch_dec = NumberOfSlotsPerEpoch::try_from(cp).unwrap();

            number_of_slots_per_epoch == number_of_slots_per_epoch_dec
        }
    }
}
