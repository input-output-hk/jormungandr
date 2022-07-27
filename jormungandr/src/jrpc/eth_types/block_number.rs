use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, num::TryFromIntError};

/// Represents rpc api block number param.
#[derive(Debug, PartialEq, Eq)]
pub enum BlockNumber {
    /// Number
    Num(u32),
    /// Latest block
    Latest,
    /// Earliest block (genesis)
    Earliest,
    /// Pending block (being mined)
    Pending,
}

impl Default for BlockNumber {
    fn default() -> Self {
        BlockNumber::Latest
    }
}

impl<'a> Deserialize<'a> for BlockNumber {
    fn deserialize<D>(deserializer: D) -> Result<BlockNumber, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(BlockNumberVisitor)
    }
}

struct BlockNumberVisitor;

impl<'a> Visitor<'a> for BlockNumberVisitor {
    type Value = BlockNumber;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a numeric block number or 'latest', 'earliest' or 'pending'"
        )
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match value {
            "latest" => Ok(BlockNumber::Latest),
            "earliest" => Ok(BlockNumber::Earliest),
            "pending" => Ok(BlockNumber::Pending),
            _ if value.starts_with("0x") => u32::from_str_radix(&value[2..], 16)
                .map(BlockNumber::Num)
                .map_err(|e| Error::custom(format!("Invalid block number: {}", e))),
            _ => value.parse::<u32>().map(BlockNumber::Num).map_err(|_| {
                Error::custom("Invalid block number: non-decimal or missing 0x prefix".to_string())
            }),
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(value.as_ref())
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(BlockNumber::Num(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(BlockNumber::Num(value.try_into().map_err(
            |e: TryFromIntError| Error::custom(e.to_string()),
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_number_json_deserialize() {
        let bn_dec: BlockNumber = serde_json::from_str(r#""42""#).unwrap();
        let bn_hex: BlockNumber = serde_json::from_str(r#""0x45""#).unwrap();
        let bn_u64: BlockNumber = serde_json::from_str(r#"420"#).unwrap();

        assert_eq!(bn_dec, BlockNumber::Num(42));
        assert_eq!(bn_hex, BlockNumber::Num(69));
        assert_eq!(bn_u64, BlockNumber::Num(420));

        let bn_latest: BlockNumber = serde_json::from_str(r#""latest""#).unwrap();
        let bn_earliest: BlockNumber = serde_json::from_str(r#""earliest""#).unwrap();
        let bn_pending: BlockNumber = serde_json::from_str(r#""pending""#).unwrap();

        assert_eq!(bn_latest, BlockNumber::Latest);
        assert_eq!(bn_earliest, BlockNumber::Earliest);
        assert_eq!(bn_pending, BlockNumber::Pending);
    }
}
