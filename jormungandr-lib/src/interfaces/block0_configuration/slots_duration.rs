use crate::{
    interfaces::{DEFAULT_SLOT_DURATION, MAXIMUM_SLOT_DURATION, MINIMUM_SLOT_DURATION},
    time::Duration,
};
use chain_impl_mockchain::config::ConfigParam;
use serde::{Deserialize, Deserializer, Serialize};
use std::{convert::TryFrom, fmt, str::FromStr as _};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct SlotDuration(u8);

impl SlotDuration {
    /// minimal value for the slot duration
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::SlotDuration;
    ///
    /// assert_eq!(SlotDuration::MINIMUM, SlotDuration::new(1).unwrap())
    /// ```
    pub const MINIMUM: Self = Self(MINIMUM_SLOT_DURATION);
    /// maximum value for the slot duration
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::SlotDuration;
    ///
    /// assert_eq!(SlotDuration::MAXIMUM, SlotDuration::new(255).unwrap())
    /// ```
    pub const MAXIMUM: Self = Self(MAXIMUM_SLOT_DURATION);

    /// create a new SlotDuration value
    ///
    /// returns `None` if the value is not within the boundaries of
    /// `SlotDuration::MINIMUM` and `SlotDuration::MAXIMUM`.
    #[allow(clippy::absurd_extreme_comparisons)]
    pub fn new(v: u8) -> Option<Self> {
        if !(MINIMUM_SLOT_DURATION..=MAXIMUM_SLOT_DURATION).contains(&v) {
            None
        } else {
            Some(Self(v))
        }
    }
}

#[derive(Debug, Error)]
pub enum TryFromSlotDurationError {
    #[error("Incompatible Config param, expected slot duration")]
    Incompatible,
    #[error("Invalid slot duration {duration}")]
    Invalid { duration: u8 },
}

impl TryFrom<ConfigParam> for SlotDuration {
    type Error = TryFromSlotDurationError;
    fn try_from(config_param: ConfigParam) -> Result<Self, Self::Error> {
        match config_param {
            ConfigParam::SlotDuration(duration) => {
                SlotDuration::new(duration).ok_or(TryFromSlotDurationError::Invalid { duration })
            }
            _ => Err(TryFromSlotDurationError::Incompatible),
        }
    }
}

impl From<SlotDuration> for ConfigParam {
    fn from(slots_duration: SlotDuration) -> Self {
        ConfigParam::SlotDuration(slots_duration.0)
    }
}

impl From<SlotDuration> for u8 {
    fn from(slots_duration: SlotDuration) -> u8 {
        slots_duration.0
    }
}

impl fmt::Display for SlotDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Duration::new(self.0 as u64, 0).fmt(f)
    }
}

impl Default for SlotDuration {
    fn default() -> Self {
        SlotDuration::new(DEFAULT_SLOT_DURATION)
            .expect("Default should be a valid value at all time")
    }
}

impl<'de> Deserialize<'de> for SlotDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        struct SlotDurationVisitor;
        impl<'de> Visitor<'de> for SlotDurationVisitor {
            type Value = SlotDuration;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "number of seconds between the creation of 2 blocks (between {} ({}) and {} ({}))",
                    MINIMUM_SLOT_DURATION, SlotDuration::MINIMUM,
                    MAXIMUM_SLOT_DURATION, SlotDuration::MAXIMUM,
                )
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v < (MINIMUM_SLOT_DURATION as u64) {
                    Err(E::custom(format!(
                        "cannot have less than {} ({}) between 2 slots",
                        MINIMUM_SLOT_DURATION,
                        SlotDuration::MINIMUM
                    )))
                } else if v > (MAXIMUM_SLOT_DURATION as u64) {
                    Err(E::custom(format!(
                        "cannot have more than {} ({}) between 2 slots",
                        MAXIMUM_SLOT_DURATION,
                        SlotDuration::MAXIMUM
                    )))
                } else {
                    Ok(SlotDuration(v as u8))
                }
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let duration = Duration::from_str(s).map_err(E::custom)?;

                if duration.as_ref().subsec_nanos() != 0 {
                    return Err(E::custom("sub-seconds not supported in slot duration"));
                }

                let seconds = duration.as_ref().as_secs();
                self.visit_u64(seconds)
            }
        }
        deserializer.deserialize_any(SlotDurationVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for SlotDuration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand07::Rng as _;
            let v = g.gen_range(MINIMUM_SLOT_DURATION, MAXIMUM_SLOT_DURATION);
            SlotDuration(v)
        }
    }

    #[test]
    #[should_panic]
    fn deserialize_from_invalid_type() {
        const EXAMPLE: &str = "---\ntrue";

        let _: SlotDuration = serde_yaml::from_str(EXAMPLE).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_from_below_bounds() {
        const VALUE: u8 = MINIMUM_SLOT_DURATION - 1;
        let example = format!("---\n{}", VALUE);

        let _: SlotDuration = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_from_above_bounds() {
        const VALUE: u64 = (MAXIMUM_SLOT_DURATION as u64) + 1;
        let example = format!("---\n{}", VALUE);

        let _: SlotDuration = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    fn deserialize_from_number() {
        const VALUE: u8 = 15;
        let example = format!("---\n{}", VALUE);

        let decoded: SlotDuration = serde_yaml::from_str(&example).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    #[test]
    fn deserialize_from_duration_str() {
        const VALUE: u8 = 15;
        const DURATION_STR: &str = "---\n15s";

        let decoded: SlotDuration = serde_yaml::from_str(DURATION_STR).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    quickcheck! {
        fn serde_encode_decode(slot_duration: SlotDuration) -> bool {
            let s = serde_yaml::to_string(&slot_duration).unwrap();
            let slot_duration_dec: SlotDuration = serde_yaml::from_str(&s).unwrap();

            slot_duration == slot_duration_dec
        }

        fn convert_from_to_config_param(slot_duration: SlotDuration) -> bool {
            let cp = ConfigParam::from(slot_duration);
            let slot_duration_dec = SlotDuration::try_from(cp).unwrap();

            slot_duration == slot_duration_dec
        }
    }
}
