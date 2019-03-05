use crate::hex;
use rand_core::{CryptoRng, RngCore};
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

    fn generate<T: RngCore + CryptoRng>(rng: T) -> Self::Secret;

    fn compute_public(secret: &Self::Secret) -> Self::Public;

    fn secret_from_binary(data: &[u8]) -> Result<Self::Secret, SecretKeyError>;
    fn public_from_binary(data: &[u8]) -> Result<Self::Public, PublicKeyError>;
}

pub struct SecretKey<A: AsymmetricKey>(pub(crate) A::Secret);

pub struct PublicKey<A: AsymmetricKey>(pub(crate) A::Public);

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

impl<A: AsymmetricKey> SecretKey<A> {
    pub fn to_public(&self) -> PublicKey<A> {
        PublicKey(<A as AsymmetricKey>::compute_public(&self.0))
    }
    pub fn from_binary(data: &[u8]) -> Result<Self, SecretKeyError> {
        Ok(SecretKey(<A as AsymmetricKey>::secret_from_binary(data)?))
    }
}

impl<A: AsymmetricKey> PublicKey<A> {
    pub fn from_binary(data: &[u8]) -> Result<Self, PublicKeyError> {
        Ok(PublicKey(<A as AsymmetricKey>::public_from_binary(data)?))
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
