//! Generic Genesis data
use cardano::util::hex;
use chain_addr::AddressReadable;
use chain_impl_mockchain::{key, transaction::Value};

use serde;
use serde_yaml;
use std::{error, fmt, io, time};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialUTxO {
    pub address: AddressReadable,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey(key::PublicKey);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisData {
    pub start_time: time::SystemTime,
    pub slot_duration: time::Duration,
    pub epoch_stability_depth: usize,
    pub initial_utxos: Vec<InitialUTxO>,
    pub obft_leaders: Vec<PublicKey>,
}

// TODO: details
#[derive(Debug)]
pub struct ParseError();

impl error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error parsing genesis data")
    }
}

impl std::str::FromStr for PublicKey {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        cardano::redeem::PublicKey::from_hex(s)
            .map(|pk| PublicKey(pk.into()))
            .map_err(|err| format!("{}", err))
    }
}

impl GenesisData {
    pub fn parse<R: io::BufRead>(reader: R) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_reader(reader)
    }

    pub fn leaders<'a>(&'a self) -> impl Iterator<Item = &'a key::PublicKey> {
        self.obft_leaders.iter().map(|pk| &pk.0)
    }
}

impl serde::ser::Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        if serializer.is_human_readable() {
            let hex = hex::encode(self.0.as_ref());
            serializer.serialize_str(&hex)
        } else {
            serializer.serialize_bytes(self.0.as_ref())
        }
    }
}
impl serde::ser::Serialize for InitialUTxO {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("InitialUTxO", 2)?;
        state.serialize_field("address", self.address.as_string())?;
        state.serialize_field("value", &self.value.0)?;
        state.end()
    }
}

impl<'de> serde::de::Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct PublicKeyVisitor;
        impl<'de> serde::de::Visitor<'de> for PublicKeyVisitor {
            type Value = PublicKey;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "PublicKey of {} bytes", 32)
            }

            fn visit_str<'a, E>(self, v: &'a str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use chain_core::property::Deserialize;
                let bytes = match hex::decode(v) {
                    Err(err) => return Err(E::custom(format!("{}", err))),
                    Ok(bytes) => bytes,
                };

                let reader = std::io::Cursor::new(bytes);
                match key::PublicKey::deserialize(reader) {
                    Err(err) => Err(E::custom(format!("{}", err))),
                    Ok(key) => Ok(PublicKey(key)),
                }
            }

            fn visit_bytes<'a, E>(self, v: &'a [u8]) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use chain_core::property::Deserialize;
                let reader = std::io::Cursor::new(v);
                match key::PublicKey::deserialize(reader) {
                    Err(err) => Err(E::custom(format!("{}", err))),
                    Ok(key) => Ok(PublicKey(key)),
                }
            }
        }
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(PublicKeyVisitor)
        } else {
            deserializer.deserialize_bytes(PublicKeyVisitor)
        }
    }
}
impl<'de> serde::de::Deserialize<'de> for InitialUTxO {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
        const FIELDS: &'static [&'static str] = &["address", "value"];

        enum Field {
            Address,
            Value,
        };

        struct InitialUTxOVisitor;

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`address` or `value`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "address" => Ok(Field::Address),
                            "value" => Ok(Field::Value),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                impl<'de> Visitor<'de> for InitialUTxOVisitor {
                    type Value = InitialUTxO;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("struct Duration")
                    }

                    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
                    where
                        V: SeqAccess<'de>,
                    {
                        let address = seq
                            .next_element()?
                            .map(|s: String| AddressReadable::from_string(&s))
                            .ok_or_else(|| de::Error::invalid_length(0, &self))?
                            .map_err(de::Error::custom)?;
                        let value = seq
                            .next_element()?
                            .map(Value)
                            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                        Ok(InitialUTxO { address, value })
                    }

                    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
                    where
                        V: MapAccess<'de>,
                    {
                        let mut address = None;
                        let mut value = None;
                        while let Some(key) = map.next_key()? {
                            match key {
                                Field::Address => {
                                    if address.is_some() {
                                        return Err(de::Error::duplicate_field("address"));
                                    }
                                    address = Some({
                                        let value = map.next_value::<String>()?;
                                        AddressReadable::from_string(&value)
                                            .map_err(de::Error::custom)?
                                    });
                                }
                                Field::Value => {
                                    if value.is_some() {
                                        return Err(de::Error::duplicate_field("value"));
                                    }
                                    value = Some(map.next_value().map(Value)?);
                                }
                            }
                        }
                        let address = address.ok_or_else(|| de::Error::missing_field("address"))?;
                        let value = value.ok_or_else(|| de::Error::missing_field("value"))?;
                        Ok(InitialUTxO { address, value })
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }
        deserializer.deserialize_struct("InitialUTxO", FIELDS, InitialUTxOVisitor)
    }
}
