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

pub mod value {
    use super::*;
    use chain_impl_mockchain::value::Value;
    use serde::de::Deserialize as _;

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
    use chain_addr::{Address, AddressReadable};

    pub fn serialize<S>(address: &Address, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let address = AddressReadable::from_address(address);
            serializer.serialize_str(address.as_string())
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
            deserializer.deserialize_str(StrParseVisitor::new("address", |s| {
                s.parse().map(|addr: AddressReadable| addr.to_address())
            }))
        } else {
            deserializer.deserialize_bytes(AddressVisitor)
        }
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

pub mod witness {
    use super::*;
    use chain_core::{
        mempack::{ReadBuf, Readable as _},
        property::Serialize as _,
    };
    use chain_impl_mockchain::transaction::Witness;
    use serde::ser::Error as _;

    pub fn serialize<S>(witness: &Witness, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = witness
            .serialize_as_vec()
            .map_err(|err| S::Error::custom(err))?;

        if serializer.is_human_readable() {
            use bech32::{Bech32, ToBase32 as _};
            let bech32 = Bech32::new("witness".to_owned(), bytes.to_base32())
                .map_err(|err| S::Error::custom(err))?;
            serializer.serialize_str(&bech32.to_string())
        } else {
            serializer.serialize_bytes(&bytes)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Witness, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = if deserializer.is_human_readable() {
            deserializer.deserialize_str(BytesInBech32Visitor::new("witness"))?
        } else {
            Vec::deserialize(deserializer)?
        };

        let mut reader = ReadBuf::from(&bytes);
        Witness::read(&mut reader).map_err(D::Error::custom)
    }
}

pub mod crypto {
    use super::*;
    use ::bech32::{Bech32 as Bech32Data, FromBase32 as _};
    use chain_crypto::{AsymmetricKey, AsymmetricPublicKey, Blake2b256, PublicKey, SecretKey};

    pub fn deserialize_secret<'de, D, A>(deserializer: D) -> Result<SecretKey<A>, D::Error>
    where
        D: Deserializer<'de>,
        A: AsymmetricKey,
    {
        let secret_key_visitor = SecretKeyVisitor::new();
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(secret_key_visitor)
        } else {
            deserializer.deserialize_bytes(secret_key_visitor)
        }
    }

    pub fn deserialize_public<'de, D, A>(deserializer: D) -> Result<PublicKey<A>, D::Error>
    where
        D: Deserializer<'de>,
        A: AsymmetricPublicKey,
    {
        let public_key_visitor = PublicKeyVisitor::new();
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(public_key_visitor)
        } else {
            deserializer.deserialize_bytes(public_key_visitor)
        }
    }

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

    struct SecretKeyVisitor<A: AsymmetricKey> {
        _marker: std::marker::PhantomData<A>,
    }
    struct PublicKeyVisitor<A: AsymmetricPublicKey> {
        _marker: std::marker::PhantomData<A>,
    }
    impl<A: AsymmetricKey> SecretKeyVisitor<A> {
        fn new() -> Self {
            SecretKeyVisitor {
                _marker: std::marker::PhantomData,
            }
        }
    }
    impl<A: AsymmetricPublicKey> PublicKeyVisitor<A> {
        fn new() -> Self {
            PublicKeyVisitor {
                _marker: std::marker::PhantomData,
            }
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

    impl<'de, A> Visitor<'de> for SecretKeyVisitor<A>
    where
        A: AsymmetricKey,
    {
        type Value = SecretKey<A>;

        fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            write!(
                fmt,
                "Expecting a secret key for algorithm {}",
                A::SECRET_BECH32_HRP
            )
        }

        fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
        where
            E: DeserializerError,
        {
            use chain_crypto::bech32::Error as Bech32Error;
            match Self::Value::try_from_bech32_str(&v) {
                Err(Bech32Error::DataInvalid(err)) => {
                    Err(E::custom(format!("Invalid data: {}", err)))
                }
                Err(Bech32Error::HrpInvalid { expected, actual }) => Err(E::custom(format!(
                    "Invalid prefix: expected {} but was {}",
                    expected, actual
                ))),
                Err(Bech32Error::Bech32Malformed(err)) => {
                    Err(E::custom(format!("Invalid bech32: {}", err)))
                }
                Ok(key) => Ok(key),
            }
        }

        fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
        where
            E: DeserializerError,
        {
            use chain_crypto::SecretKeyError;
            match Self::Value::from_binary(v) {
                Err(SecretKeyError::SizeInvalid) => Err(E::custom("Invalid size")),
                Err(SecretKeyError::StructureInvalid) => Err(E::custom("Invalid structure")),
                Ok(key) => Ok(key),
            }
        }
    }

    impl<'de, A> Visitor<'de> for PublicKeyVisitor<A>
    where
        A: AsymmetricPublicKey,
    {
        type Value = PublicKey<A>;

        fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            write!(
                fmt,
                "Expecting a public key for algorithm {}",
                A::PUBLIC_BECH32_HRP
            )
        }

        fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
        where
            E: DeserializerError,
        {
            use chain_crypto::bech32::Error as Bech32Error;
            match Self::Value::try_from_bech32_str(&v) {
                Err(Bech32Error::DataInvalid(err)) => {
                    Err(E::custom(format!("Invalid data: {}", err)))
                }
                Err(Bech32Error::HrpInvalid { expected, actual }) => Err(E::custom(format!(
                    "Invalid prefix: expected {} but was {}",
                    expected, actual
                ))),
                Err(Bech32Error::Bech32Malformed(err)) => {
                    Err(E::custom(format!("Invalid bech32: {}", err)))
                }
                Ok(key) => Ok(key),
            }
        }

        fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
        where
            E: DeserializerError,
        {
            use chain_crypto::PublicKeyError;
            match Self::Value::from_binary(v) {
                Err(PublicKeyError::SizeInvalid) => Err(E::custom(format!(
                    "Invalid size (expected: {}bytes)",
                    A::PUBLIC_KEY_SIZE
                ))),
                Err(PublicKeyError::StructureInvalid) => Err(E::custom("Invalid structure")),
                Ok(key) => Ok(key),
            }
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

pub mod certificate {

    use super::*;
    use chain_impl_mockchain::certificate::Certificate;

    pub fn serialize<S>(cert: &Certificate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use crate::certificate as cert;
        use serde::ser::Error as _;
        let bech32 = cert::serialize_to_bech32(cert).map_err(|err| S::Error::custom(err))?;
        serializer.serialize_str(&bech32.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Certificate, D::Error>
    where
        D: Deserializer<'de>,
    {
        use chain_core::mempack::{ReadBuf, Readable};
        deserializer
            .deserialize_str(BytesInBech32Visitor::new("cert"))
            .and_then(|bytes| {
                let mut buf = ReadBuf::from(&bytes);
                Certificate::read(&mut buf).map_err(|err| D::Error::custom(err))
            })
    }
}

pub mod system_time {
    use super::*;
    use std::time::SystemTime;

    pub fn serialize<S>(timestamp: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        humantime::format_rfc3339_nanos(*timestamp)
            .to_string()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SystemTime, D::Error> {
        let visitor = StrParseVisitor::new("RFC3339 timestamp", humantime::parse_rfc3339_weak);
        deserializer.deserialize_str(visitor)
    }
}

#[derive(Serialize)]
#[serde(bound = "T: ToString", transparent)]
pub struct SerdeAsString<T>(#[serde(with = "as_string")] pub T);

impl<'de, E: Display, T: FromStr<Err = E> + SerdeExpected> Deserialize<'de> for SerdeAsString<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        as_string::deserialize(deserializer).map(Self)
    }
}

impl<T: Clone> Clone for SerdeAsString<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(
    bound(deserialize = "T: Bech32 + SerdeExpected", serialize = "T: Bech32"),
    transparent
)]
pub struct SerdeAsBech32<T>(#[serde(with = "as_bech32")] pub T);

impl<T: Clone> Clone for SerdeAsBech32<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
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
