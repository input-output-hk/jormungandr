use chain_impl_mockchain::value;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

/// Value in the blockchain, always printed as absolute Lovelace
///
/// Value has some property to be human readable on standard display
///
/// ```
/// # use jormungandr_lib::interfaces::Value;
/// # use chain_impl_mockchain::value::Value as StdValue;
///
/// let value: Value = StdValue(64).into();
///
/// println!("value: {}", value);
///
/// # assert_eq!(value.to_string(), "64");
/// ```
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value(value::Value);

/* ---------------- Display ------------------------------------------------ */

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Value {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(|v| Value(value::Value(v)))
    }
}

/* ---------------- AsRef -------------------------------------------------- */

impl AsRef<value::Value> for Value {
    fn as_ref(&self) -> &value::Value {
        &self.0
    }
}
/* ---------------- Conversion --------------------------------------------- */

impl From<value::Value> for Value {
    fn from(v: value::Value) -> Self {
        Value(v)
    }
}

impl From<Value> for value::Value {
    fn from(v: Value) -> Self {
        v.0
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_ref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = u64::deserialize(deserializer)?;
        Ok(Value(value::Value(v)))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for Value {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            Value(value::Value(u64::arbitrary(g)))
        }
    }

    #[test]
    fn value_display_as_u64() {
        const VALUE: u64 = 928170;
        let value = Value(value::Value(VALUE));

        assert_eq!(value.to_string(), VALUE.to_string());
    }

    #[test]
    fn value_serde_as_u64() {
        const VALUE: u64 = 928170;
        let value = Value(value::Value(VALUE));

        assert_eq!(
            serde_yaml::to_string(&value).unwrap(),
            format!("---\n{}", VALUE)
        );
    }

    quickcheck! {
        fn value_display_parse(value: Value) -> TestResult {
            let s = value.to_string();
            let value_dec: Value = s.parse().unwrap();

            TestResult::from_bool(value_dec == value)
        }

        fn value_serde_human_readable_encode_decode(value: Value) -> TestResult {
            let s = serde_yaml::to_string(&value).unwrap();
            let value_dec: Value = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(value_dec == value)
        }

        fn value_serde_binary_encode_decode(value: Value) -> TestResult {
            let s = bincode::serialize(&value).unwrap();
            let value_dec: Value = bincode::deserialize(&s).unwrap();

            TestResult::from_bool(value_dec == value)
        }
    }
}
