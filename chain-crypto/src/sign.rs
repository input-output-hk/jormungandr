use crate::{hex, key};
use std::fmt;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verification {
    Failed,
    Success,
}

impl From<bool> for Verification {
    fn from(b: bool) -> Self {
        if b {
            Verification::Success
        } else {
            Verification::Failed
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SignatureError {
    SizeInvalid,
    StructureInvalid,
}

pub trait VerificationAlgorithm: key::AsymmetricKey {
    type Signature: AsRef<[u8]> + Clone;

    fn verify_bytes(pubkey: &Self::Public, signature: &Self::Signature, msg: &[u8])
        -> Verification;

    fn signature_from_bytes(data: &[u8]) -> Result<Self::Signature, SignatureError>;
}

pub trait SigningAlgorithm: VerificationAlgorithm {
    fn sign(key: &Self::Secret, msg: &[u8]) -> Self::Signature;
}

pub struct Signature<T, A: VerificationAlgorithm> {
    signdata: A::Signature,
    phantom: PhantomData<T>,
}

impl<A: VerificationAlgorithm, T> fmt::Debug for Signature<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.signdata.as_ref()))
    }
}
impl<A: VerificationAlgorithm, T> fmt::Display for Signature<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.signdata.as_ref()))
    }
}
impl fmt::Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SignatureError::SizeInvalid => write!(f, "Invalid Signature size"),
            SignatureError::StructureInvalid => write!(f, "Invalid Signature structure"),
        }
    }
}

impl std::error::Error for SignatureError {}

impl<A: VerificationAlgorithm, T> Signature<T, A> {
    pub fn from_bytes(sig: &[u8]) -> Result<Self, SignatureError> {
        Ok(Signature {
            signdata: A::signature_from_bytes(sig)?,
            phantom: PhantomData,
        })
    }
    pub fn coerce<U>(self) -> Signature<U, A> {
        Signature {
            signdata: self.signdata,
            phantom: PhantomData,
        }
    }
}

impl<A: VerificationAlgorithm, T: AsRef<[u8]>> Signature<T, A> {
    pub fn verify(&self, publickey: &key::PublicKey<A>, object: &T) -> Verification {
        <A as VerificationAlgorithm>::verify_bytes(&publickey.0, &self.signdata, object.as_ref())
    }
}

impl<A: SigningAlgorithm, T: AsRef<[u8]>> Signature<T, A> {
    pub fn generate(secretkey: &key::SecretKey<A>, object: &T) -> Signature<T, A> {
        Signature {
            signdata: <A as SigningAlgorithm>::sign(&secretkey.0, object.as_ref()),
            phantom: PhantomData,
        }
    }
}

impl<T, A: VerificationAlgorithm> Clone for Signature<T, A> {
    fn clone(&self) -> Self {
        Signature {
            signdata: self.signdata.clone(),
            phantom: std::marker::PhantomData,
        }
    }
}
