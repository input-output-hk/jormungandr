use chain_impl_mockchain::{config::ConfigParam, milli::Milli};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{convert::TryFrom, fmt, str::FromStr as _};

const DEFAULT_BFT_SLOTS_RATIO: u64 = 0_220;
const MINIMUM_BFT_SLOTS_RATIO: u64 = 0_000;
const MAXIMUM_BFT_SLOTS_RATIO: u64 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BFTSlotsRatio(pub(crate) Milli);

impl BFTSlotsRatio {
    /// minimal value for the BFT slot ratio
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::BFTSlotsRatio;
    /// # use chain_impl_mockchain::milli::Milli;
    ///
    /// assert_eq!(BFTSlotsRatio::MINIMUM, BFTSlotsRatio::new(Milli::ZERO).unwrap())
    /// ```
    pub const MINIMUM: Self = BFTSlotsRatio(Milli::from_millis(MINIMUM_BFT_SLOTS_RATIO));

    /// maximal value for the bft slot ratio
    ///
    /// ```
    /// # use jormungandr_lib::interfaces::BFTSlotsRatio;
    /// # use chain_impl_mockchain::milli::Milli;
    ///
    /// assert_eq!(BFTSlotsRatio::MAXIMUM, BFTSlotsRatio::new(Milli::ONE).unwrap())
    /// ```
    pub const MAXIMUM: Self = BFTSlotsRatio(Milli::from_millis(MAXIMUM_BFT_SLOTS_RATIO));

    pub fn new(milli: Milli) -> Option<Self> {
        if milli.to_millis() < MINIMUM_BFT_SLOTS_RATIO
            || MAXIMUM_BFT_SLOTS_RATIO < milli.to_millis()
        {
            None
        } else {
            Some(BFTSlotsRatio(milli))
        }
    }
}

custom_error! { pub TryFromBFTSlotsRatioError
    Incompatible = "Incompatible Config param, expected BFT slots ratio",
    Invalid { ratio: Milli } = "invalid BFT slots ratio {ratio}",
}

impl TryFrom<ConfigParam> for BFTSlotsRatio {
    type Error = TryFromBFTSlotsRatioError;
    fn try_from(config_param: ConfigParam) -> Result<Self, Self::Error> {
        match config_param {
            ConfigParam::BftSlotsRatio(ratio) => {
                BFTSlotsRatio::new(ratio).ok_or(TryFromBFTSlotsRatioError::Invalid { ratio })
            }
            _ => Err(TryFromBFTSlotsRatioError::Incompatible),
        }
    }
}

impl From<BFTSlotsRatio> for ConfigParam {
    fn from(bft_slots_ratio: BFTSlotsRatio) -> Self {
        ConfigParam::BftSlotsRatio(bft_slots_ratio.0)
    }
}

impl fmt::Display for BFTSlotsRatio {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for BFTSlotsRatio {
    fn default() -> Self {
        BFTSlotsRatio::new(Milli::from_millis(DEFAULT_BFT_SLOTS_RATIO))
            .expect("Default should be a valid value at all time")
    }
}

impl Serialize for BFTSlotsRatio {
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

impl<'de> Deserialize<'de> for BFTSlotsRatio {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        struct BFTSlotsRatioVisitor;
        impl<'de> Visitor<'de> for BFTSlotsRatioVisitor {
            type Value = BFTSlotsRatio;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "BFT slots ratio within range of {} and {}",
                    BFTSlotsRatio::MINIMUM,
                    BFTSlotsRatio::MAXIMUM,
                )
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v == 1 {
                    Ok(BFTSlotsRatio(Milli::ONE))
                } else {
                    Err(E::custom(format!(
                        "value out of bound, can only accept within range of {} and {}",
                        BFTSlotsRatio::MINIMUM,
                        BFTSlotsRatio::MAXIMUM,
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

                if milli < MINIMUM_BFT_SLOTS_RATIO {
                    Err(E::custom(format!(
                        "cannot have BFT slots ratio below {}",
                        BFTSlotsRatio::MINIMUM,
                    )))
                } else if MAXIMUM_BFT_SLOTS_RATIO < milli {
                    Err(E::custom(format!(
                        "cannot have BFT slots ratio above {}",
                        BFTSlotsRatio::MAXIMUM,
                    )))
                } else {
                    Ok(BFTSlotsRatio(Milli::from_millis(milli)))
                }
            }
        }

        deserializer.deserialize_any(BFTSlotsRatioVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for BFTSlotsRatio {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand::Rng as _;
            let v = g.gen_range(MINIMUM_BFT_SLOTS_RATIO, MAXIMUM_BFT_SLOTS_RATIO);
            BFTSlotsRatio(Milli::from_millis(v))
        }
    }

    #[test]
    #[should_panic]
    fn deserialize_from_invalid_type() {
        const EXAMPLE: &'static str = "---\ntrue";

        let _: BFTSlotsRatio = serde_yaml::from_str(EXAMPLE).unwrap();
    }

    /// this test is ignored for as long as MINIMUM_BFT_SLOTS_RATIO is set
    /// to Milli::ZERO
    #[test]
    #[should_panic]
    #[ignore]
    fn deserialize_from_below_bounds() {
        const VALUE: u64 = MINIMUM_BFT_SLOTS_RATIO;
        let example = format!("---\n{}", VALUE);

        let _: BFTSlotsRatio = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_from_above_bounds() {
        const VALUE: u64 = MAXIMUM_BFT_SLOTS_RATIO + 1;
        let example = format!("---\n{}", VALUE);

        let _: BFTSlotsRatio = serde_yaml::from_str(&example).unwrap();
    }

    #[test]
    fn deserialize_from_float() {
        const VALUE: Milli = Milli::from_millis(500);
        let example = format!("---\n{}", VALUE);

        let decoded: BFTSlotsRatio = serde_yaml::from_str(&example).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    #[test]
    fn deserialize_from_number() {
        const VALUE: Milli = Milli::ONE;
        let example = format!("---\n{}", 1);

        let decoded: BFTSlotsRatio = serde_yaml::from_str(&example).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    #[test]
    fn deserialize_from_str() {
        const VALUE: Milli = Milli::from_millis(220);
        const ACTIVE_SLOT_STR: &'static str = "---\n\"0.220\"";

        let decoded: BFTSlotsRatio = serde_yaml::from_str(&ACTIVE_SLOT_STR).unwrap();

        assert_eq!(decoded.0, VALUE)
    }

    quickcheck! {
        fn serde_encode_decode(active_slot_coefficient: BFTSlotsRatio) -> bool {
            let s = serde_yaml::to_string(&active_slot_coefficient).unwrap();
            let active_slot_coefficient_dec: BFTSlotsRatio = serde_yaml::from_str(&s).unwrap();

            active_slot_coefficient == active_slot_coefficient_dec
        }

        fn convert_from_to_config_param(bft_slots_ratio: BFTSlotsRatio) -> bool {
            let cp = ConfigParam::from(bft_slots_ratio);
            let bft_slots_ratio_dec = BFTSlotsRatio::try_from(cp).unwrap();

            bft_slots_ratio == bft_slots_ratio_dec
        }
    }
}
