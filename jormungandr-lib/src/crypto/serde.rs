use chain_crypto::bech32::Bech32;
use chain_crypto::{AsymmetricKey, AsymmetricPublicKey, Blake2b256, PublicKey, SecretKey};
use serde::{
    de::{Deserializer, Error as DeserializerError, Visitor},
    ser::Serializer,
    Serialize,
};
use std::fmt;
use std::str::FromStr;

pub fn serialize_secret<S, A>(key: &SecretKey<A>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    A: AsymmetricKey,
{
    if serializer.is_human_readable() {
        key.to_bech32_str().serialize(serializer)
    } else {
        panic!("binary encoding for serialization of the secret key does not exist in chain-crypto")
    }
}

pub fn serialize_public<S, A>(key: &PublicKey<A>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    A: AsymmetricPublicKey,
{
    if serializer.is_human_readable() {
        key.to_bech32_str().serialize(serializer)
    } else {
        key.as_ref().serialize(serializer)
    }
}

pub fn serialize_hash<S>(hash: &Blake2b256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if serializer.is_human_readable() {
        hash.to_string().serialize(serializer)
    } else {
        hash.as_ref().serialize(serializer)
    }
}

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
    let hash_visitor = HashVisitor::new();
    if deserializer.is_human_readable() {
        deserializer.deserialize_str(hash_visitor)
    } else {
        deserializer.deserialize_bytes(hash_visitor)
    }
}

struct HashVisitor;
struct SecretKeyVisitor<A: AsymmetricKey> {
    _marker: std::marker::PhantomData<A>,
}
struct PublicKeyVisitor<A: AsymmetricPublicKey> {
    _marker: std::marker::PhantomData<A>,
}
impl HashVisitor {
    fn new() -> Self {
        HashVisitor
    }
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
            Err(Bech32Error::DataInvalid(err)) => Err(E::custom(format!("Invalid data: {}", err))),
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
            Err(Bech32Error::DataInvalid(err)) => Err(E::custom(format!("Invalid data: {}", err))),
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

impl<'de> Visitor<'de> for HashVisitor {
    type Value = Blake2b256;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Expecting a Blake2b256 Hash",)
    }

    fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
    where
        E: DeserializerError,
    {
        Blake2b256::from_str(v).map_err(E::custom)
    }

    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
    where
        E: DeserializerError,
    {
        Blake2b256::try_from_slice(v).map_err(E::custom)
    }
}
