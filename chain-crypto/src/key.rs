use crate::bech32::{self, Bech32};
use crate::hex;
use rand::{CryptoRng, RngCore};
use std::borrow::Cow;
use std::fmt;
use std::hash::Hash;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SecretKeyError {
    SizeInvalid,
    StructureInvalid,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PublicKeyError {
    SizeInvalid,
    StructureInvalid,
}

pub trait AsymmetricKey {
    type Secret: AsRef<[u8]> + Clone;
    type Public: AsRef<[u8]> + Clone + PartialEq + Eq + Hash;

    const SECRET_BECH32_HRP: &'static str;
    const PUBLIC_BECH32_HRP: &'static str;

    const SECRET_KEY_SIZE: usize;
    const PUBLIC_KEY_SIZE: usize;

    fn generate<T: RngCore + CryptoRng>(rng: T) -> Self::Secret;

    fn compute_public(secret: &Self::Secret) -> Self::Public;

    fn secret_from_binary(data: &[u8]) -> Result<Self::Secret, SecretKeyError>;
    fn public_from_binary(data: &[u8]) -> Result<Self::Public, PublicKeyError>;
}

pub struct SecretKey<A: AsymmetricKey>(pub(crate) A::Secret);

pub struct PublicKey<A: AsymmetricKey>(pub(crate) A::Public);

pub struct KeyPair<A: AsymmetricKey>(SecretKey<A>, PublicKey<A>);

impl<A: AsymmetricKey> KeyPair<A> {
    pub fn private_key(&self) -> &SecretKey<A> {
        &self.0
    }
    pub fn public_key(&self) -> &PublicKey<A> {
        &self.1
    }
    pub fn into_keys(self) -> (SecretKey<A>, PublicKey<A>) {
        (self.0, self.1)
    }
}
impl<A: AsymmetricKey> std::fmt::Debug for KeyPair<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "KeyPair(<secret key>, {:?})", self.public_key())
    }
}
impl<A: AsymmetricKey> std::fmt::Display for KeyPair<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "KeyPair(<secret key>, {})", self.public_key())
    }
}

impl<A: AsymmetricKey> fmt::Debug for PublicKey<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0.as_ref()))
    }
}
impl<A: AsymmetricKey> fmt::Display for PublicKey<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0.as_ref()))
    }
}
impl fmt::Display for SecretKeyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SecretKeyError::SizeInvalid => write!(f, "Invalid Secret Key size"),
            SecretKeyError::StructureInvalid => write!(f, "Invalid Secret Key structure"),
        }
    }
}
impl fmt::Display for PublicKeyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PublicKeyError::SizeInvalid => write!(f, "Invalid Public Key size"),
            PublicKeyError::StructureInvalid => write!(f, "Invalid Public Key structure"),
        }
    }
}
impl std::error::Error for SecretKeyError {}
impl std::error::Error for PublicKeyError {}

impl<A: AsymmetricKey> AsRef<[u8]> for PublicKey<A> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<A: AsymmetricKey> From<SecretKey<A>> for KeyPair<A> {
    fn from(secret_key: SecretKey<A>) -> Self {
        let public_key = secret_key.to_public();
        KeyPair(secret_key, public_key)
    }
}

impl<A: AsymmetricKey> SecretKey<A> {
    pub fn generate<T: RngCore + CryptoRng>(rng: T) -> Self {
        SecretKey(A::generate(rng))
    }
    pub fn to_public(&self) -> PublicKey<A> {
        PublicKey(<A as AsymmetricKey>::compute_public(&self.0))
    }
    pub fn from_binary(data: &[u8]) -> Result<Self, SecretKeyError> {
        Ok(SecretKey(<A as AsymmetricKey>::secret_from_binary(data)?))
    }
    pub fn from_bytes(data: &[u8]) -> Result<Self, SecretKeyError> {
        Self::from_binary(data)
    }
}

impl<A: AsymmetricKey> PublicKey<A> {
    pub fn from_binary(data: &[u8]) -> Result<Self, PublicKeyError> {
        Ok(PublicKey(<A as AsymmetricKey>::public_from_binary(data)?))
    }
    pub fn from_bytes(data: &[u8]) -> Result<Self, PublicKeyError> {
        Self::from_binary(data)
    }
}

impl<A: AsymmetricKey> Clone for SecretKey<A> {
    fn clone(&self) -> Self {
        SecretKey(self.0.clone())
    }
}
impl<A: AsymmetricKey> Clone for PublicKey<A> {
    fn clone(&self) -> Self {
        PublicKey(self.0.clone())
    }
}
impl<A: AsymmetricKey> Clone for KeyPair<A> {
    fn clone(&self) -> Self {
        KeyPair(self.0.clone(), self.1.clone())
    }
}

impl<A: AsymmetricKey> std::cmp::PartialEq<Self> for PublicKey<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref().eq(other.0.as_ref())
    }
}

impl<A: AsymmetricKey> std::cmp::Eq for PublicKey<A> {}

impl<A: AsymmetricKey> std::cmp::PartialOrd<Self> for PublicKey<A> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.as_ref().partial_cmp(other.0.as_ref())
    }
}

impl<A: AsymmetricKey> std::cmp::Ord for PublicKey<A> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.as_ref().cmp(other.0.as_ref())
    }
}

impl<A: AsymmetricKey> Hash for PublicKey<A> {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.0.as_ref().hash(state)
    }
}

impl<A: AsymmetricKey> Bech32 for PublicKey<A> {
    const BECH32_HRP: &'static str = A::PUBLIC_BECH32_HRP;

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, bech32::Error> {
        Self::from_bytes(bytes).map_err(bech32::Error::data_invalid)
    }

    fn to_bytes(&self) -> Cow<[u8]> {
        self.as_ref().into()
    }
}

impl<A: AsymmetricKey> Bech32 for SecretKey<A> {
    const BECH32_HRP: &'static str = A::SECRET_BECH32_HRP;

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, bech32::Error> {
        Self::from_bytes(bytes).map_err(|e| bech32::Error::DataInvalid(Box::new(e)))
    }

    fn to_bytes(&self) -> Cow<[u8]> {
        self.0.as_ref().into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use quickcheck::{Arbitrary, Gen};
    use rand::SeedableRng;
    use rand_chacha::ChaChaRng;

    pub fn arbitrary_public_key<A: AsymmetricKey, G: Gen>(g: &mut G) -> PublicKey<A> {
        arbitrary_secret_key(g).to_public()
    }

    pub fn arbitrary_secret_key<A, G>(g: &mut G) -> SecretKey<A>
    where
        A: AsymmetricKey,
        G: Gen,
    {
        let rng = ChaChaRng::seed_from_u64(Arbitrary::arbitrary(g));
        SecretKey::generate(rng)
    }

    impl<A> Arbitrary for PublicKey<A>
    where
        A: AsymmetricKey + 'static,
        A::Public: Send,
    {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            arbitrary_public_key(g)
        }
    }
    impl<A> Arbitrary for SecretKey<A>
    where
        A: AsymmetricKey + 'static,
        A::Secret: Send,
    {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            arbitrary_secret_key(g)
        }
    }
    impl<A> Arbitrary for KeyPair<A>
    where
        A: AsymmetricKey + 'static,
        A::Secret: Send,
        A::Public: Send,
    {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let secret_key = SecretKey::arbitrary(g);
            KeyPair::from(secret_key)
        }
    }
}
