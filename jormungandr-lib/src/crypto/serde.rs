use chain_crypto::{
    bech32::{Bech32, Error as Bech32Error},
    AsymmetricKey, AsymmetricPublicKey, Blake2b256, PublicKey, SecretKey, Signature,
    VerificationAlgorithm,
};
use serde::{
    de::{Deserializer, Error as DeserializerError, Visitor},
    ser::Serializer,
    Serialize,
};
use std::{fmt, str::FromStr};

pub fn serialize_secret<S, A>(key: &SecretKey<A>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    A: AsymmetricKey,
    SecretKey<A>: Bech32,
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

pub fn serialize_signature<S, T, A>(
    signature: &Signature<T, A>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    A: VerificationAlgorithm,
{
    if serializer.is_human_readable() {
        signature.to_bech32_str().serialize(serializer)
    } else {
        signature.as_ref().serialize(serializer)
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
    SecretKey<A>: Bech32,
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

pub fn deserialize_signature<'de, D, T, A>(deserializer: D) -> Result<Signature<T, A>, D::Error>
where
    D: Deserializer<'de>,
    A: VerificationAlgorithm,
{
    let signature_visitor = SignatureVisitor::new();
    if deserializer.is_human_readable() {
        deserializer.deserialize_str(signature_visitor)
    } else {
        deserializer.deserialize_bytes(signature_visitor)
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
struct SignatureVisitor<T, A: VerificationAlgorithm> {
    _marker_1: std::marker::PhantomData<T>,
    _marker_2: std::marker::PhantomData<A>,
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
impl<T, A: VerificationAlgorithm> SignatureVisitor<T, A> {
    fn new() -> Self {
        SignatureVisitor {
            _marker_1: std::marker::PhantomData,
            _marker_2: std::marker::PhantomData,
        }
    }
}

impl<'de, A> Visitor<'de> for SecretKeyVisitor<A>
where
    A: AsymmetricKey,
    SecretKey<A>: Bech32,
{
    type Value = SecretKey<A>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "a secret key for algorithm {}", A::SECRET_BECH32_HRP)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: DeserializerError,
    {
        Self::Value::try_from_bech32_str(v).map_err(bech32_error_to_serde)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
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
        write!(fmt, "a public key for algorithm {}", A::PUBLIC_BECH32_HRP)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: DeserializerError,
    {
        Self::Value::try_from_bech32_str(v).map_err(bech32_error_to_serde)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
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

impl<'de, T, A> Visitor<'de> for SignatureVisitor<T, A>
where
    A: VerificationAlgorithm,
{
    type Value = Signature<T, A>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "a signature for algorithm {}", A::SIGNATURE_BECH32_HRP)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: DeserializerError,
    {
        Self::Value::try_from_bech32_str(v).map_err(bech32_error_to_serde)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: DeserializerError,
    {
        use chain_crypto::SignatureError;
        match Self::Value::from_binary(v) {
            Err(SignatureError::SizeInvalid { expected, got }) => Err(E::custom(format!(
                "Invalid size (expected: {}bytes but received {}bytes)",
                expected, got,
            ))),
            Err(SignatureError::StructureInvalid) => Err(E::custom("Invalid structure")),
            Ok(key) => Ok(key),
        }
    }
}

impl<'de> Visitor<'de> for HashVisitor {
    type Value = Blake2b256;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "a Blake2b256 Hash",)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: DeserializerError,
    {
        Blake2b256::from_str(v).map_err(E::custom)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: DeserializerError,
    {
        Blake2b256::try_from_slice(v).map_err(E::custom)
    }
}

fn bech32_error_to_serde<E>(error: Bech32Error) -> E
where
    E: DeserializerError,
{
    match error {
        Bech32Error::DataInvalid(err) => E::custom(format!("Invalid data: {}", err)),
        Bech32Error::HrpInvalid { expected, actual } => E::custom(format!(
            "Invalid prefix: expected {} but was {}",
            expected, actual
        )),
        Bech32Error::Bech32Malformed(err) => E::custom(format!("Invalid bech32: {}", err)),
        Bech32Error::UnexpectedDataLen { expected, actual } => E::custom(format!(
            "Invalid bech32 length: expected {} but was actual {}",
            expected, actual
        )),
    }
}
