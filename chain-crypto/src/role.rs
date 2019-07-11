use crate::key;
use rand_core::{CryptoRng, RngCore};
use std::hash::Hash;
use std::marker::PhantomData;

pub struct SecretKey<R, Algorithm: key::AsymmetricKey> {
    inner: key::SecretKey<Algorithm>,
    marker: PhantomData<R>,
}

pub struct PublicKey<R, Algorithm: key::AsymmetricPublicKey> {
    inner: key::PublicKey<Algorithm>,
    marker: PhantomData<R>,
}

impl<R, Algorithm: key::AsymmetricKey> SecretKey<R, Algorithm> {
    pub fn role(sk: key::SecretKey<Algorithm>) -> Self {
        SecretKey {
            inner: sk,
            marker: PhantomData,
        }
    }

    pub fn unrole(self) -> key::SecretKey<Algorithm> {
        self.inner
    }

    pub fn generate<T: RngCore + CryptoRng>(rng: T) -> Self {
        Self::role(key::SecretKey::generate(rng))
    }

    pub fn to_public(&self) -> PublicKey<R, Algorithm::PubAlg> {
        PublicKey::role(self.inner.to_public())
    }
}

impl<R, Algorithm: key::AsymmetricPublicKey> PublicKey<R, Algorithm> {
    pub fn role(pk: key::PublicKey<Algorithm>) -> Self {
        PublicKey {
            inner: pk,
            marker: PhantomData,
        }
    }

    pub fn unrole(self) -> key::PublicKey<Algorithm> {
        self.inner
    }
}

impl<R, A: key::AsymmetricKey> Clone for SecretKey<R, A> {
    fn clone(&self) -> Self {
        SecretKey {
            inner: self.inner.clone(),
            marker: self.marker.clone(),
        }
    }
}

impl<R, A: key::AsymmetricPublicKey> Clone for PublicKey<R, A> {
    fn clone(&self) -> Self {
        PublicKey {
            inner: self.inner.clone(),
            marker: self.marker,
        }
    }
}

impl<R, A: key::AsymmetricPublicKey> std::cmp::PartialEq<Self> for PublicKey<R, A> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<R, A: key::AsymmetricPublicKey> std::cmp::Eq for PublicKey<R, A> {}

impl<R, A: key::AsymmetricPublicKey> std::cmp::PartialOrd<Self> for PublicKey<R, A> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<R, A: key::AsymmetricPublicKey> std::cmp::Ord for PublicKey<R, A> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<R, A: key::AsymmetricPublicKey> Hash for PublicKey<R, A> {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.inner.hash(state)
    }
}
