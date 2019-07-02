//! module that contains all the serde functionalities
//! to serialize and deserialize most of the objects
//!

use chain_crypto::{bech32::Bech32, Ed25519, PublicKey};
use chain_impl_mockchain::leadership::bft::LeaderId;
use serde::{
    de::{Deserializer, Error as DeserializerError, Visitor},
    ser::Serializer,
    Deserialize, Serialize,
};
use std::fmt::{self, Display};
use std::str::FromStr;

pub mod crypto {
    use super::*;
    use ::bech32::{Bech32 as Bech32Data, FromBase32 as _};
    use chain_crypto::Blake2b256;

    pub fn deserialize_hash<'de, D>(deserializer: D) -> Result<Blake2b256, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            as_string::deserialize(deserializer)
        } else {
            unimplemented!()
        }
    }

    pub fn deserialize_bench32<'de, D>(
        deserializer: D,
        hrp: &'static str,
    ) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secret_key_visitor = BytesInBech32Visitor::new(hrp);
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(secret_key_visitor)
        } else {
            deserializer.deserialize_bytes(secret_key_visitor)
        }
    }

    pub struct BytesInBech32Visitor {
        hrp: &'static str,
    }

    impl BytesInBech32Visitor {
        pub fn new(hrp: &'static str) -> Self {
            BytesInBech32Visitor { hrp }
        }
    }

    impl<'de> Visitor<'de> for BytesInBech32Visitor {
        type Value = Vec<u8>;

        fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            write!(fmt, "Expecting bech32 data with HRP {}", self.hrp)
        }

        fn visit_str<'a, E>(self, bech32_str: &'a str) -> Result<Self::Value, E>
        where
            E: DeserializerError,
        {
            let bech32: Bech32Data = bech32_str
                .parse()
                .map_err(|err| E::custom(format!("Invalid bech32: {}", err)))?;
            if bech32.hrp() != self.hrp {
                return Err(E::custom(format!(
                    "Invalid prefix: expected {} but was {}",
                    self.hrp,
                    bech32.hrp()
                )));
            }
            let bytes = Vec::<u8>::from_base32(bech32.data())
                .map_err(|err| E::custom(format!("Invalid bech32: {}", err)))?;
            Ok(bytes)
        }
    }
}

#[derive(Clone, Debug)]
pub struct SerdeLeaderId(pub LeaderId);

impl Serialize for SerdeLeaderId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        as_bech32::serialize(self.0.as_public_key(), serializer)
    }
}

impl<'de> Deserialize<'de> for SerdeLeaderId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        as_bech32::deserialize::<D, PublicKey<Ed25519>>(deserializer)
            .map(|key| SerdeLeaderId(key.into()))
    }
}

pub struct BytesInBech32Visitor {
    hrp: &'static str,
}

impl BytesInBech32Visitor {
    pub fn new(hrp: &'static str) -> Self {
        BytesInBech32Visitor { hrp }
    }
}

impl<'de> Visitor<'de> for BytesInBech32Visitor {
    type Value = Vec<u8>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Expecting bech32 data with HRP {}", self.hrp)
    }

    fn visit_str<'a, E>(self, bech32_str: &'a str) -> Result<Self::Value, E>
    where
        E: DeserializerError,
    {
        use bech32::{Bech32, FromBase32};
        let bech32: Bech32 = bech32_str
            .parse()
            .map_err(|err| E::custom(format!("Invalid bech32: {}", err)))?;
        if bech32.hrp() != self.hrp {
            return Err(E::custom(format!(
                "Invalid prefix: expected {} but was {}",
                self.hrp,
                bech32.hrp()
            )));
        }
        let bytes = Vec::<u8>::from_base32(bech32.data())
            .map_err(|err| E::custom(format!("Invalid bech32: {}", err)))?;
        Ok(bytes)
    }
}

pub mod as_string {
    use super::*;

    pub fn serialize<S: Serializer, T: ToString>(
        data: &T,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        data.to_string().serialize(serializer)
    }

    pub fn deserialize<'de, D, E, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        E: Display,
        T: FromStr<Err = E> + SerdeExpected,
    {
        let visitor = StrParseVisitor::new(T::EXPECTED, str::parse);
        deserializer.deserialize_str(visitor)
    }
}

pub mod as_bech32 {
    use super::*;
    use chain_crypto::bech32::Bech32;

    pub fn serialize<S: Serializer, T: Bech32>(data: &T, serializer: S) -> Result<S::Ok, S::Error> {
        data.to_bech32_str().serialize(serializer)
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Bech32 + SerdeExpected,
    {
        let visitor = StrParseVisitor::new(T::EXPECTED, Bech32::try_from_bech32_str);
        deserializer.deserialize_str(visitor)
    }
}

#[derive(Default)]
struct StrParseVisitor<'a, P> {
    expected: &'a str,
    parser: P,
}

impl<'a, E, T, P> StrParseVisitor<'a, P>
where
    E: Display,
    P: FnOnce(&str) -> Result<T, E>,
{
    pub fn new(expected: &'a str, parser: P) -> Self {
        Self { expected, parser }
    }
}

impl<'a, 'de, E, T, P> Visitor<'de> for StrParseVisitor<'a, P>
where
    E: Display,
    P: FnOnce(&str) -> Result<T, E>,
{
    type Value = T;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.expected)
    }

    fn visit_str<D: DeserializerError>(self, s: &str) -> Result<Self::Value, D> {
        (self.parser)(s).map_err(D::custom)
    }
}

// used to generate deserialize error messages telling what data was expected
pub trait SerdeExpected {
    const EXPECTED: &'static str;
}

impl SerdeExpected for chain_impl_mockchain::milli::Milli {
    const EXPECTED: &'static str = "floating point number in decimal form";
}

impl SerdeExpected for chain_crypto::PublicKey<chain_crypto::Ed25519> {
    const EXPECTED: &'static str = "ED25519 public key";
}

impl SerdeExpected for chain_addr::Discrimination {
    const EXPECTED: &'static str = "address discrimination";
}

impl SerdeExpected for chain_impl_mockchain::block::ConsensusVersion {
    const EXPECTED: &'static str = "consensus version";
}

impl SerdeExpected for chain_crypto::Blake2b256 {
    const EXPECTED: &'static str = "Blake2b 256";
}
