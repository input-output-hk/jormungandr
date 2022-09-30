use crate::{
    interfaces::{
        DEFAULT_KES_SPEED_UPDATE, MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS,
        MINIMUM_KES_SPEED_UPDATE_IN_SECONDS,
    },
    time::Duration,
};
use chain_impl_mockchain::config::ConfigParam;
use serde::{Deserialize, Deserializer, Serialize};
use std::{convert::TryFrom, fmt, str::FromStr as _};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct KesUpdateSpeed(u32);

impl KesUpdateSpeed {
    /// minimal value for the KES Update Speed
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::KesUpdateSpeed;
    ///
    /// assert_eq!(KesUpdateSpeed::MINIMUM, KesUpdateSpeed::new(60).unwrap())
    /// ```
    pub const MINIMUM: Self = KesUpdateSpeed(MINIMUM_KES_SPEED_UPDATE_IN_SECONDS);

    /// maximum value for the KES Update Speed
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::KesUpdateSpeed;
    ///
    /// assert_eq!(KesUpdateSpeed::MAXIMUM, KesUpdateSpeed::new(365 * 24 * 3600).unwrap())
    /// ```
    pub const MAXIMUM: Self = KesUpdateSpeed(MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS);

    /// create a new KesUpdateSpeed value
    ///
    /// returns `None` if the value is not within the boundaries of
    /// `KesUpdateSpeed::MINIMUM` and `KesUpdateSpeed::MAXIMUM`.
    pub fn new(v: u32) -> Option<Self> {
        if !(MINIMUM_KES_SPEED_UPDATE_IN_SECONDS..=MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS).contains(&v)
        {
            None
        } else {
            Some(KesUpdateSpeed(v))
        }
    }
}

#[derive(Debug, Error)]
pub enum TryFromKesUpdateSpeedError {
    #[error("Incompatible Config param, expected KES Update Speed")]
    Incompatible,
    #[error("Invalid KES Update speed {speed}")]
    Invalid { speed: u32 },
}

impl TryFrom<ConfigParam> for KesUpdateSpeed {
    type Error = TryFromKesUpdateSpeedError;
    fn try_from(config_param: ConfigParam) -> Result<Self, Self::Error> {
        match config_param {
            ConfigParam::KesUpdateSpeed(speed) => {
                KesUpdateSpeed::new(speed).ok_or(TryFromKesUpdateSpeedError::Invalid { speed })
            }
            _ => Err(TryFromKesUpdateSpeedError::Incompatible),
        }
    }
}

impl From<KesUpdateSpeed> for ConfigParam {
    fn from(kes_update_speed: KesUpdateSpeed) -> Self {
        ConfigParam::KesUpdateSpeed(kes_update_speed.0)
    }
}

impl fmt::Display for KesUpdateSpeed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Duration::new(self.0 as u64, 0).fmt(f)
    }
}

impl Default for KesUpdateSpeed {
    fn default() -> Self {
        KesUpdateSpeed::new(DEFAULT_KES_SPEED_UPDATE)
            .expect("Default should be a valid value at all time")
    }
}

impl<'de> Deserialize<'de> for KesUpdateSpeed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        struct KesUpdateSpeedVisitor;
        impl<'de> Visitor<'de> for KesUpdateSpeedVisitor {
            type Value = KesUpdateSpeed;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "number of seconds between 2 KES updates (valid values are between {} ({}) and {} ({}))",
                    MINIMUM_KES_SPEED_UPDATE_IN_SECONDS, KesUpdateSpeed::MINIMUM,
                    MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS, KesUpdateSpeed::MAXIMUM,
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
                        KesUpdateSpeed::MINIMUM
                    )))
                } else if v > (MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS as u64) {
                    Err(E::custom(format!(
                        "cannot have more than {} ({}) between two KES Update",
                        MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS,
                        KesUpdateSpeed::MAXIMUM
                    )))
                } else {
                    Ok(KesUpdateSpeed(v as u32))
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
        deserializer.deserialize_any(KesUpdateSpeedVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for KesUpdateSpeed {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand07::Rng as _;
            let v = g.gen_range(
                MINIMUM_KES_SPEED_UPDATE_IN_SECONDS,
                MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS,
            );
            KesUpdateSpeed(v)
        }
    }

    #[test]
    #[should_panic]
    fn deserialize_from_invalid_type() {
        const EXAMPLE: &str = "---\ntrue";

        let _: KesUpdateSpeed = serde_yaml::from_str(EXAMPLE).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_from_below_bounds() {
        const VALUE: u32 = MINIMUM_KES_SPEED_UPDATE_IN_SECONDS - 1;
        let example = format!("---\n{}", VALUE);

        let _: KesUpdateSpeed = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_from_above_bounds() {
        const VALUE: u32 = MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS + 1;
        let example = format!("---\n{}", VALUE);

        let _: KesUpdateSpeed = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    fn deserialize_from_number() {
        const VALUE: u32 = 92827;
        let example = format!("---\n{}", VALUE);

        let decoded: KesUpdateSpeed = serde_yaml::from_str(&example).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    #[test]
    fn deserialize_from_duration_str() {
        const VALUE: u32 = 2 * 24 * 3600 + 6 * 3600 + 15 * 60 + 34;
        const DURATION_STR: &str = "---\n2days 6h 15m 34s";

        let decoded: KesUpdateSpeed = serde_yaml::from_str(DURATION_STR).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    quickcheck! {
        fn serde_encode_decode(kes_update_speed: KesUpdateSpeed) -> bool {
            let s = serde_yaml::to_string(&kes_update_speed).unwrap();
            let kes_update_speed_dec: KesUpdateSpeed = serde_yaml::from_str(&s).unwrap();

            kes_update_speed == kes_update_speed_dec
        }

        fn convert_from_to_config_param(kes_update_speed: KesUpdateSpeed) -> bool {
            let cp = ConfigParam::from(kes_update_speed);
            let kes_update_speed_dec = KesUpdateSpeed::try_from(cp).unwrap();

            kes_update_speed == kes_update_speed_dec
        }
    }
}
