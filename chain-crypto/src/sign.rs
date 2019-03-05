use crate::{hex, key};
use std::fmt;
use std::marker::PhantomData;

#[derive(Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl<A: VerificationAlgorithm, T: AsRef<[u8]>> Signature<T, A> {
    pub fn verify(
        publickey: &key::PublicKey<A>,
        object: &T,
        signature: &Signature<T, A>,
    ) -> Verification {
        <A as VerificationAlgorithm>::verify_bytes(
            &publickey.0,
            &signature.signdata,
            object.as_ref(),
        )
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
