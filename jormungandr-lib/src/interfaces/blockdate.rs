use chain_impl_mockchain::block::{self, Epoch, SlotId};
use chain_time::TimeEra;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockDate(block::BlockDate);

impl BlockDate {
    pub fn new(epoch: Epoch, slot_id: SlotId) -> Self {
        Self(block::BlockDate { epoch, slot_id })
    }

    pub fn next_epoch(self) -> Self {
        self.0.next_epoch().into()
    }

    pub fn next(self, era: &TimeEra) -> Self {
        self.0.next(era).into()
    }

    pub fn slot(&self) -> SlotId {
        self.0.slot_id
    }

    pub fn epoch(&self) -> Epoch {
        self.0.epoch
    }

    pub fn time_era(&self, slots_in_epoch: u32) -> TimeEra {
        TimeEra::new(
            (self.0.slot_id as u64).into(),
            chain_time::Epoch(self.0.epoch),
            slots_in_epoch,
        )
    }

    pub fn shift_epoch(&self, epoch_shift: u32) -> Self {
        let mut block_date: block::BlockDate = (*self).into();
        for _ in 0..epoch_shift {
            block_date = block_date.next_epoch();
        }
        block_date.into()
    }

    pub fn shift_slot(&self, slot_shift: u32, time_era: &TimeEra) -> Self {
        let mut block_date: block::BlockDate = (*self).into();
        for _ in 0..slot_shift {
            block_date = block_date.next(&time_era);
        }
        block_date.into()
    }
}

/* ---------------- Display ------------------------------------------------ */

impl fmt::Display for BlockDate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for BlockDate {
    type Err = block::BlockDateParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(BlockDate)
    }
}

/* ---------------- AsRef -------------------------------------------------- */

impl AsRef<block::BlockDate> for BlockDate {
    fn as_ref(&self) -> &block::BlockDate {
        &self.0
    }
}

/* ---------------- Conversion --------------------------------------------- */

impl From<block::BlockDate> for BlockDate {
    fn from(block_date: block::BlockDate) -> Self {
        BlockDate(block_date)
    }
}

impl From<BlockDate> for block::BlockDate {
    fn from(block_date: BlockDate) -> Self {
        block_date.0
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for BlockDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            self.to_string().serialize(serializer)
        } else {
            (self.0.epoch, self.0.slot_id).serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for BlockDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct StringOrStruct;
        impl<'de> Visitor<'de> for StringOrStruct {
            type Value = BlockDate;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, value: &str) -> Result<BlockDate, E>
            where
                E: de::Error,
            {
                let bd = BlockDate::from_str(value).map_err(E::custom)?;
                Ok(bd)
            }

            fn visit_map<M>(self, map: M) -> Result<BlockDate, M::Error>
            where
                M: MapAccess<'de>,
            {
                let d = de::value::MapAccessDeserializer::new(map);
                let bd = BlockDateStructural::deserialize(d)?;
                Ok(BlockDate(bd))
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_any(StringOrStruct)
        } else {
            let (epoch, slot_id): (u32, u32) = Deserialize::deserialize(deserializer)?;
            Ok(BlockDate(block::BlockDate { epoch, slot_id }))
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "chain_impl_mockchain::block::BlockDate")]
struct BlockDateStructural {
    pub epoch: u32,
    pub slot_id: u32,
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for BlockDate {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            BlockDate(block::BlockDate {
                epoch: Arbitrary::arbitrary(g),
                slot_id: Arbitrary::arbitrary(g),
            })
        }
    }

    #[test]
    fn display_expected_value() {
        let date = BlockDate(block::BlockDate {
            epoch: 12,
            slot_id: 928,
        });

        assert_eq!(date.to_string(), "12.928")
    }

    quickcheck! {
        fn display_and_from_str(date: BlockDate) -> TestResult {
            let encoded = date.to_string();
            let decoded : BlockDate = match BlockDate::from_str(&encoded) {
                Err(err) => return TestResult::error(err.to_string()),
                Ok(v) => v
            };

            TestResult::from_bool(decoded == date)
        }

        fn serde_human_readable_encode_decode(date: BlockDate) -> TestResult {
            let encoded = serde_yaml::to_string(&date).unwrap();
            let decoded : BlockDate = match serde_yaml::from_str(&encoded) {
                Err(err) => return TestResult::error(err.to_string()),
                Ok(v) => v
            };

            TestResult::from_bool(decoded == date)
        }

        fn serde_binary_encode_decode(date: BlockDate) -> TestResult {
            let encoded = bincode::serialize(&date).unwrap();
            let decoded : BlockDate = match bincode::deserialize(&encoded) {
                Err(err) => return TestResult::error(err.to_string()),
                Ok(v) => v
            };

            TestResult::from_bool(decoded == date)
        }
    }
}
