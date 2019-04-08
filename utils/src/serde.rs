//! module that contains all the serde functionalities
//! to serialize and deserialize most of the objects
//!

use serde::{
    de::{Deserialize, Deserializer, Error as DeserializerError, Visitor},
    ser::{Error as SerializerError, Serialize, Serializer},
};
use std::fmt;

pub mod value {
    use super::*;
    use chain_impl_mockchain::value::Value;

    pub fn serialize<S>(value: &Value, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.0.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(Value)
    }
}

pub mod address {
    use super::*;
    use chain_addr::{Address, AddressReadable, Discrimination};

    pub fn serialize<S>(address: &Address, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let address = AddressReadable::from_address(address);
            serialize_readable(&address, serializer)
        } else {
            let bytes = address.to_bytes();
            serializer.serialize_bytes(&bytes)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Address, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer
                .deserialize_str(FromStrVisitor::new("Address"))
                .map(|address_readable: AddressReadable| address_readable.to_address())
        } else {
            deserializer.deserialize_bytes(AddressVisitor)
        }
    }

    pub fn serialize_discrimination<S>(
        discrimination: &Discrimination,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&discrimination.to_string())
    }

    pub fn deserialize_discrimination<'de, D>(deserializer: D) -> Result<Discrimination, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(FromStrVisitor::new("Address Discrimination"))
    }

    pub fn serialize_readable<S>(
        address: &AddressReadable,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(address.as_string())
    }

    pub fn deserialize_readable<'de, D>(deserializer: D) -> Result<AddressReadable, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(FromStrVisitor::new("A bech32 encoded address"))
    }

    struct AddressVisitor;

    impl<'de> Visitor<'de> for AddressVisitor {
        type Value = Address;

        fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            write!(fmt, "Expecting an Address",)
        }

        fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
        where
            E: DeserializerError,
        {
            use chain_core::mempack::{ReadBuf, Readable};
            let mut buf = ReadBuf::from(v);
            match Self::Value::read(&mut buf) {
                Err(err) => Err(E::custom(err)),
                Ok(address) => Ok(address),
            }
        }
    }

}

pub mod block {
    use super::*;
    use chain_impl_mockchain::block::ConsensusVersion;

    pub fn serialize_consensus_version<S>(
        version: &ConsensusVersion,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&version.to_string())
    }

    pub fn deserialize_consensus_version<'de, D>(
        deserializer: D,
    ) -> Result<ConsensusVersion, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(FromStrVisitor::new("consensus version"))
    }
}

pub mod time {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize_system_time_in_sec<S>(
        time: &SystemTime,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(time.duration_since(UNIX_EPOCH).unwrap().as_secs())
    }

    pub fn deserialize_system_time_in_sec<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let duration_since_unix_epoch = deserializer.deserialize_u64(DurationVisitor)?;
        let time = SystemTime::UNIX_EPOCH + duration_since_unix_epoch;
        Ok(time)
    }

    struct DurationVisitor;
    impl<'de> Visitor<'de> for DurationVisitor {
        type Value = Duration;

        fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            write!(fmt, "Expecting a duration in seconds")
        }
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: DeserializerError,
        {
            Ok(Duration::from_secs(v))
        }
    }
}

pub(crate) struct FromStrVisitor<A> {
    pub(crate) what: &'static str,
    pub(crate) _marker: std::marker::PhantomData<A>,
}
impl<A> FromStrVisitor<A> {
    pub(crate) fn new(what: &'static str) -> Self {
        FromStrVisitor {
            what,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'de, A> Visitor<'de> for FromStrVisitor<A>
where
    A: std::str::FromStr,
    A::Err: std::error::Error,
{
    type Value = A;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Expecting a {}", self.what)
    }

    fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
    where
        E: DeserializerError,
    {
        match Self::Value::from_str(v) {
            Err(err) => Err(E::custom(err)),
            Ok(address) => Ok(address),
        }
    }
}
