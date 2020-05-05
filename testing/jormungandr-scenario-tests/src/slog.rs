use bytes::BytesMut;
use serde_json::{Map, Value};
use std::convert::TryFrom;
use tokio::codec::Decoder;

#[derive(Debug, Clone)]
pub struct StructuredLog {
    level: slog::Level,
    message: String,

    full_log: serde_json::Map<String, Value>,
}

/// Structural log decoder
///
/// For the testing purpose of jormungandr we will assume the structural
/// logs to always be in JSON output format
///
pub struct SlogCodec(tokio::codec::LinesCodec);

error_chain! {
    foreign_links {
        Io(::std::io::Error);
    }

    errors {
        InvalidJSON {
            description("Invalid log format, expected JSON object"),
            display("Invalid log format, expected JSON object"),
        }

        FieldNotFound(key: String) {
            description("Field not found"),
            display("Field not found in the SLOG: {}", key),
        }

        InvalidValue(value: Value) {
            description("Cannot parse value"),
            display("cannot parse value `{}`", value),
        }

        InvalidLog {
            description("Invalid log"),
            display("Invalid log format, cannot parse"),
        }
    }
}

impl TryFrom<Map<String, Value>> for StructuredLog {
    type Error = Error;

    fn try_from(map: Map<String, Value>) -> Result<Self> {
        let level = map_get(&map, "level")
            .and_then(value_is_str)
            .and_then(|s| s.parse().map_err(|()| ErrorKind::InvalidLog.into()))
            .chain_err(|| "`level`")?;
        let message = map_get(&map, "msg")
            .and_then(value_is_str)
            .and_then(|s| s.parse().chain_err(|| ErrorKind::InvalidLog))
            .chain_err(|| "`msg`")?;
        Ok(StructuredLog {
            level,
            message,
            full_log: map,
        })
    }
}

impl Decoder for SlogCodec {
    type Item = StructuredLog;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>> {
        let line = self.0.decode(buf)?;

        if let Some(line) = line {
            serde_json::from_str(&line)
                .chain_err(|| ErrorKind::InvalidJSON)
                .and_then(|map: Map<_, _>| StructuredLog::try_from(map))
                .map(Some)
        } else {
            Ok(None)
        }
    }
}

#[inline]
fn map_get<'a>(map: &'a Map<String, Value>, key: &str) -> Result<&'a Value> {
    if let Some(value) = map.get(key) {
        Ok(value)
    } else {
        Err(ErrorKind::FieldNotFound(key.to_owned()).into())
    }
}

#[inline]
fn value_is_str<'a>(value: &'a Value) -> Result<&'a str> {
    if let Value::String(string) = value {
        Ok(&string)
    } else {
        Err(ErrorKind::InvalidValue(value.to_owned()).into())
    }
}
