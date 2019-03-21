use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Display;
use std::str::FromStr;

pub fn deserialize<'de, T, E, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr<Err = E>,
    E: Display,
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(D::Error::custom)
}

pub fn serialize<T: ToString, S: Serializer>(data: &T, serializer: S) -> Result<S::Ok, S::Error> {
    data.to_string().serialize(serializer)
}
