use crate::hex;
use rand_core::{CryptoRng, RngCore};
use std::fmt;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretKeyError {
    SizeInvalid,
    StructureInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Clone)]
pub struct SecretKey<A: AsymmetricKey>(pub(crate) A::Secret);

#[derive(Clone, PartialEq, Eq, Hash)]
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
