use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

/// Represents usize.
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct Number(u64);

impl Number {
    pub fn inc(&mut self) {
        self.0 += 1;
    }
}

impl From<u64> for Number {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

impl From<Number> for u64 {
    fn from(val: Number) -> Self {
        val.0
    }
}

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(format!("{:#x}", self.0).as_str())
    }
}

impl<'a> Deserialize<'a> for Number {
    fn deserialize<D>(deserializer: D) -> Result<Number, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(NumberVisitor)
    }
}

struct NumberVisitor;

impl<'a> Visitor<'a> for NumberVisitor {
    type Value = Number;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a hex-encoded or decimal index")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match value {
            _ if value.starts_with("0x") => u64::from_str_radix(&value[2..], 16)
                .map(Number)
                .map_err(|e| Error::custom(format!("Invalid index: {}", e))),
            _ => value
                .parse::<u64>()
                .map(Number)
                .map_err(|e| Error::custom(format!("Invalid index: {}", e))),
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(value.as_ref())
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Number(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_serde() {
        let s = r#"["0xa", "10", 42, "0x45"]"#;
        let deserialized: Vec<Number> = serde_json::from_str(s).unwrap();
        assert_eq!(
            deserialized,
            vec![Number(10), Number(10), Number(42), Number(69)]
        );

        assert_eq!(
            serde_json::to_string(&vec![Number(10), Number(10), Number(42), Number(69)]).unwrap(),
            r#"["0xa","0xa","0x2a","0x45"]"#
        );
    }
}
