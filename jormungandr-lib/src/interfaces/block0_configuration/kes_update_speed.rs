use crate::time::Duration;
use chain_impl_mockchain::config::ConfigParam;
use serde::{Deserialize, Deserializer, Serialize};
use std::{convert::TryFrom, fmt, str::FromStr as _};

const DEFAULT_KES_SPEED_UPDATE: u32 = 12 * 3600;
const MINIMUM_KES_SPEED_UPDATE_IN_SECONDS: u32 = 60;
const MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS: u32 = 365 * 24 * 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct KESUpdateSpeed(pub(crate) u32);

impl KESUpdateSpeed {
    /// minimal value for the KES Update Speed
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::KESUpdateSpeed;
    ///
    /// assert_eq!(KESUpdateSpeed::MINIMUM, KESUpdateSpeed::new(60).unwrap())
    /// ```
    pub const MINIMUM: Self = KESUpdateSpeed(MINIMUM_KES_SPEED_UPDATE_IN_SECONDS);

    /// maximum value for the KES Update Speed
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::KESUpdateSpeed;
    ///
    /// assert_eq!(KESUpdateSpeed::MAXIMUM, KESUpdateSpeed::new(365 * 24 * 3600).unwrap())
    /// ```
    pub const MAXIMUM: Self = KESUpdateSpeed(MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS);

    /// create a new KESUpdateSpeed value
    ///
    /// returns `None` if the value is not within the boundaries of
    /// `KESUpdateSpeed::MINIMUM` and `KESUpdateSpeed::MAXIMUM`.
    pub fn new(v: u32) -> Option<Self> {
        if v < MINIMUM_KES_SPEED_UPDATE_IN_SECONDS || MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS < v {
            None
        } else {
            Some(KESUpdateSpeed(v))
        }
    }
}

custom_error! { pub TryFromKESUpdateSpeedError
    Incompatible = "Incompatible Config param, expected KES Update Speed",
    Invalid { speed: u32 } = "Invalid KES Update speed {speed}",
}

impl TryFrom<ConfigParam> for KESUpdateSpeed {
    type Error = TryFromKESUpdateSpeedError;
    fn try_from(config_param: ConfigParam) -> Result<Self, Self::Error> {
        match config_param {
            ConfigParam::KESUpdateSpeed(speed) => KESUpdateSpeed::new(speed)
                .ok_or(TryFromKESUpdateSpeedError::Invalid { speed }),
            _ => Err(TryFromKESUpdateSpeedError::Incompatible),
        }
    }
}

impl From<KESUpdateSpeed> for ConfigParam {
    fn from(kes_update_speed: KESUpdateSpeed) -> Self {
        ConfigParam::KESUpdateSpeed(kes_update_speed.0)
    }
}

impl fmt::Display for KESUpdateSpeed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Duration::new(self.0 as u64, 0).fmt(f)
    }
}

impl Default for KESUpdateSpeed {
    fn default() -> Self {
        KESUpdateSpeed::new(DEFAULT_KES_SPEED_UPDATE)
            .expect("Default should be a valid value at all time")
    }
}

impl<'de> Deserialize<'de> for KESUpdateSpeed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        struct KESUpdateSpeedVisitor;
        impl<'de> Visitor<'de> for KESUpdateSpeedVisitor {
            type Value = KESUpdateSpeed;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "Number of seconds between 2 KES update (valid values are between {} ({}) and {} ({}))",
                    MINIMUM_KES_SPEED_UPDATE_IN_SECONDS, KESUpdateSpeed::MINIMUM,
                    MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS, KESUpdateSpeed::MAXIMUM,
                )
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v < (MINIMUM_KES_SPEED_UPDATE_IN_SECONDS as u64) {
                    Err(E::custom(format!(
                        "cannot have less than {} ({}) between two KES Update",
                        MINIMUM_KES_SPEED_UPDATE_IN_SECONDS,
                        KESUpdateSpeed::MINIMUM
                    )))
                } else if v > (MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS as u64) {
                    Err(E::custom(format!(
                        "cannot have more than {} ({}) between two KES Update",
                        MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS,
                        KESUpdateSpeed::MAXIMUM
                    )))
                } else {
                    Ok(KESUpdateSpeed(v as u32))
                }
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let duration = Duration::from_str(s).map_err(E::custom)?;

                if duration.as_ref().subsec_nanos() != 0 {
                    return Err(E::custom("cannot sub-seconds in the KES update speed"));
                }

                let seconds = duration.as_ref().as_secs();
                self.visit_u64(seconds)
            }
        }
        deserializer.deserialize_any(KESUpdateSpeedVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for KESUpdateSpeed {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand::Rng as _;
            let v = g.gen_range(
                MINIMUM_KES_SPEED_UPDATE_IN_SECONDS,
                MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS,
            );
            KESUpdateSpeed(v)
        }
    }

    #[test]
    #[should_panic]
    fn deserialize_from_invalid_type() {
        const EXAMPLE: &'static str = "---\ntrue";

        let _: KESUpdateSpeed = serde_yaml::from_str(EXAMPLE).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_from_below_bounds() {
        const VALUE: u32 = MINIMUM_KES_SPEED_UPDATE_IN_SECONDS - 1;
        let example = format!("---\n{}", VALUE);

        let _: KESUpdateSpeed = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_from_above_bounds() {
        const VALUE: u32 = MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS + 1;
        let example = format!("---\n{}", VALUE);

        let _: KESUpdateSpeed = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    fn deserialize_from_number() {
        const VALUE: u32 = 92827;
        let example = format!("---\n{}", VALUE);

        let decoded: KESUpdateSpeed = serde_yaml::from_str(&example).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    #[test]
    fn deserialize_from_duration_str() {
        const VALUE: u32 = 2 * 24 * 3600 + 6 * 3600 + 15 * 60 + 34;
        const DURATION_STR: &'static str = "---\n2days 6h 15m 34s";

        let decoded: KESUpdateSpeed = serde_yaml::from_str(&DURATION_STR).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    quickcheck! {
        fn serde_encode_decode(kes_update_speed: KESUpdateSpeed) -> bool {
            let s = serde_yaml::to_string(&kes_update_speed).unwrap();
            let kes_update_speed_dec: KESUpdateSpeed = serde_yaml::from_str(&s).unwrap();

            kes_update_speed == kes_update_speed_dec
        }

        fn convert_from_to_config_param(kes_update_speed: KESUpdateSpeed) -> bool {
            let cp = ConfigParam::from(kes_update_speed);
            let kes_update_speed_dec = KESUpdateSpeed::try_from(cp).unwrap();

            kes_update_speed == kes_update_speed_dec
        }
    }
}
