use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bytes(Box<[u8]>);

impl From<Box<[u8]>> for Bytes {
    fn from(val: Box<[u8]>) -> Self {
        Self(val)
    }
}

impl From<Bytes> for Box<[u8]> {
    fn from(val: Bytes) -> Self {
        val.0
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialized = "0x".to_owned();
        serialized.push_str(hex::encode(&self.0).as_str());
        serializer.serialize_str(serialized.as_ref())
    }
}

impl<'a> Deserialize<'a> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(BytesVisitor)
    }
}

struct BytesVisitor;

impl<'a> Visitor<'a> for BytesVisitor {
    type Value = Bytes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a 0x-prefixed, hex-encoded vector of bytes")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if value.len() >= 2 && value.starts_with("0x") {
            Ok(Bytes(
                hex::decode(&value[2..])
                    .map_err(|e| Error::custom(format!("Invalid hex: {}", e)))?
                    .into(),
            ))
        } else {
            Err(Error::custom(
                "Invalid bytes format. Expected a 0x-prefixed hex string",
            ))
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(value.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_json_serde() {
        let bytes = Bytes([1, 2, 3, 4, 5, 69].into());
        assert_eq!(
            serde_json::to_string(&bytes).unwrap(),
            r#""0x010203040545""#
        );
        let decoded: Bytes = serde_json::from_str(r#""0x010203040545""#).unwrap();
        assert_eq!(decoded, bytes);

        let bytes = Bytes([].into());
        assert_eq!(serde_json::to_string(&bytes).unwrap(), r#""0x""#);
        let decoded: Bytes = serde_json::from_str(r#""0x""#).unwrap();
        assert_eq!(decoded, bytes);
    }
}
