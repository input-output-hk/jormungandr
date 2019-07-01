use crate::interfaces::{
    DEFAULT_ACTIVE_SLOT_COEFFICIENT, MAXIMUM_ACTIVE_SLOT_COEFFICIENT,
    MINIMUM_ACTIVE_SLOT_COEFFICIENT,
};
use chain_impl_mockchain::{config::ConfigParam, milli::Milli};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{convert::TryFrom, fmt, str::FromStr as _};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActiveSlotCoefficient(pub(crate) Milli);

impl ActiveSlotCoefficient {
    /// minimal value for the active slot coefficient
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::ActiveSlotCoefficient;
    /// # use chain_impl_mockchain::milli::Milli;
    ///
    /// assert_eq!(ActiveSlotCoefficient::MINIMUM, ActiveSlotCoefficient::new(Milli::from_millis(0_001)).unwrap())
    /// ```
    pub const MINIMUM: Self =
        ActiveSlotCoefficient(Milli::from_millis(MINIMUM_ACTIVE_SLOT_COEFFICIENT));

    /// maximal value for the active slot coefficient
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::ActiveSlotCoefficient;
    /// # use chain_impl_mockchain::milli::Milli;
    ///
    /// assert_eq!(ActiveSlotCoefficient::MAXIMUM, ActiveSlotCoefficient::new(Milli::from_millis(1_000)).unwrap())
    /// ```
    pub const MAXIMUM: Self =
        ActiveSlotCoefficient(Milli::from_millis(MAXIMUM_ACTIVE_SLOT_COEFFICIENT));

    pub fn new(milli: Milli) -> Option<Self> {
        if milli.to_millis() < MINIMUM_ACTIVE_SLOT_COEFFICIENT
            || MAXIMUM_ACTIVE_SLOT_COEFFICIENT < milli.to_millis()
        {
            None
        } else {
            Some(ActiveSlotCoefficient(milli))
        }
    }
}

custom_error! { pub TryFromActiveSlotCoefficientError
    Incompatible = "Incompatible Config param, expected active slot coefficient",
    Invalid { coefficient: Milli } = "invalid active slot coefficient {coefficient}",
}

impl TryFrom<ConfigParam> for ActiveSlotCoefficient {
    type Error = TryFromActiveSlotCoefficientError;
    fn try_from(config_param: ConfigParam) -> Result<Self, Self::Error> {
        match config_param {
            ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(coefficient) => {
                ActiveSlotCoefficient::new(coefficient)
                    .ok_or(TryFromActiveSlotCoefficientError::Invalid { coefficient })
            }
            _ => Err(TryFromActiveSlotCoefficientError::Incompatible),
        }
    }
}

impl From<ActiveSlotCoefficient> for ConfigParam {
    fn from(active_slot_coefficient: ActiveSlotCoefficient) -> Self {
        ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(active_slot_coefficient.0)
    }
}

impl fmt::Display for ActiveSlotCoefficient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for ActiveSlotCoefficient {
    fn default() -> Self {
        ActiveSlotCoefficient::new(Milli::from_millis(DEFAULT_ACTIVE_SLOT_COEFFICIENT))
            .expect("Default should be a valid value at all time")
    }
}

impl Serialize for ActiveSlotCoefficient {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0 == Milli::ONE {
            serializer.serialize_u64(1)
        } else {
            serializer.serialize_str(&self.0.to_string())
        }
    }
}

impl<'de> Deserialize<'de> for ActiveSlotCoefficient {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        struct ActiveSlotCoefficientVisitor;
        impl<'de> Visitor<'de> for ActiveSlotCoefficientVisitor {
            type Value = ActiveSlotCoefficient;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "active slot coefficient within range of {} and {}",
                    ActiveSlotCoefficient::MINIMUM,
                    ActiveSlotCoefficient::MAXIMUM,
                )
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v == 1 {
                    Ok(ActiveSlotCoefficient(Milli::ONE))
                } else {
                    Err(E::custom(format!(
                        "value out of bound, can only accept within range of {} and {}",
                        ActiveSlotCoefficient::MINIMUM,
                        ActiveSlotCoefficient::MAXIMUM,
                    )))
                }
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&format!("{:.3}", v))
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let milli = Milli::from_str(s).map_err(E::custom)?;
                let milli = milli.to_millis();

                if milli < MINIMUM_ACTIVE_SLOT_COEFFICIENT {
                    Err(E::custom(format!(
                        "cannot have active slot coefficient below {}",
                        ActiveSlotCoefficient::MINIMUM,
                    )))
                } else if MAXIMUM_ACTIVE_SLOT_COEFFICIENT < milli {
                    Err(E::custom(format!(
                        "cannot have active slot coefficient above {}",
                        ActiveSlotCoefficient::MAXIMUM,
                    )))
                } else {
                    Ok(ActiveSlotCoefficient(Milli::from_millis(milli)))
                }
            }
        }

        deserializer.deserialize_any(ActiveSlotCoefficientVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for ActiveSlotCoefficient {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand::Rng as _;
            let v = g.gen_range(
                MINIMUM_ACTIVE_SLOT_COEFFICIENT,
                MAXIMUM_ACTIVE_SLOT_COEFFICIENT,
            );
            ActiveSlotCoefficient(Milli::from_millis(v))
        }
    }

    #[test]
    #[should_panic]
    fn deserialize_from_invalid_type() {
        const EXAMPLE: &'static str = "---\ntrue";

        let _: ActiveSlotCoefficient = serde_yaml::from_str(EXAMPLE).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_from_below_bounds() {
        const VALUE: u64 = MINIMUM_ACTIVE_SLOT_COEFFICIENT - 1;
        let example = format!("---\n{}", VALUE);

        let _: ActiveSlotCoefficient = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_from_above_bounds() {
        const VALUE: u64 = MAXIMUM_ACTIVE_SLOT_COEFFICIENT + 1;
        let example = format!("---\n{}", VALUE);

        let _: ActiveSlotCoefficient = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    fn deserialize_from_float() {
        const VALUE: Milli = Milli::from_millis(500);
        let example = format!("---\n{}", VALUE);

        let decoded: ActiveSlotCoefficient = serde_yaml::from_str(&example).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    #[test]
    fn deserialize_from_number() {
        const VALUE: Milli = Milli::ONE;
        let example = format!("---\n{}", 1);

        let decoded: ActiveSlotCoefficient = serde_yaml::from_str(&example).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    #[test]
    fn deserialize_from_str() {
        const VALUE: Milli = Milli::from_millis(220);
        const ACTIVE_SLOT_STR: &'static str = "---\n\"0.220\"";

        let decoded: ActiveSlotCoefficient = serde_yaml::from_str(&ACTIVE_SLOT_STR).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    quickcheck! {
        fn serde_encode_decode(active_slot_coefficient: ActiveSlotCoefficient) -> bool {
            let s = serde_yaml::to_string(&active_slot_coefficient).unwrap();
            let active_slot_coefficient_dec: ActiveSlotCoefficient = serde_yaml::from_str(&s).unwrap();

            active_slot_coefficient == active_slot_coefficient_dec
        }

        fn convert_from_to_config_param(active_slot_coefficient: ActiveSlotCoefficient) -> bool {
            let cp = ConfigParam::from(active_slot_coefficient);
            let active_slot_coefficient_dec = ActiveSlotCoefficient::try_from(cp).unwrap();

            active_slot_coefficient == active_slot_coefficient_dec
        }
    }
}
