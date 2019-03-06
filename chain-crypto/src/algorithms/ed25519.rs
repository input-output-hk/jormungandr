use crate::key::{AsymmetricKey, PublicKeyError, SecretKeyError};
use crate::sign::{SignatureError, SigningAlgorithm, Verification, VerificationAlgorithm};
use cryptoxide::ed25519;
use rand_core::{CryptoRng, RngCore};

/// ED25519 Signing Algorithm
pub struct Ed25519;

/// ED25519 Signing Algorithm with extended secret key
pub struct Ed25519Extended;

#[derive(Clone)]
pub struct Priv([u8; ed25519::PRIVATE_KEY_LENGTH]);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Pub(pub(crate) [u8; ed25519::PUBLIC_KEY_LENGTH]);

#[derive(Clone)]
pub struct Sig(pub(crate) [u8; ed25519::SIGNATURE_LENGTH]);

impl AsRef<[u8]> for Priv {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl AsRef<[u8]> for Pub {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for Sig {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsymmetricKey for Ed25519 {
    type Secret = Priv;
    type Public = Pub;

    fn generate<T: RngCore + CryptoRng>(mut rng: T) -> Self::Secret {
        let mut priv_bytes = [0u8; ed25519::PRIVATE_KEY_LENGTH];
        rng.fill_bytes(&mut priv_bytes);
        Priv(priv_bytes)
    }

    fn compute_public(key: &Self::Secret) -> Self::Public {
        let (_, pk) = ed25519::keypair(&key.0);
        Pub(pk)
    }

    fn secret_from_binary(data: &[u8]) -> Result<Self::Secret, SecretKeyError> {
        if data.len() != ed25519::PRIVATE_KEY_LENGTH {
            return Err(SecretKeyError::SizeInvalid);
        }
        let mut buf = [0; ed25519::PRIVATE_KEY_LENGTH];
        buf[0..ed25519::PRIVATE_KEY_LENGTH].clone_from_slice(data);
        Ok(Priv(buf))
    }
    fn public_from_binary(data: &[u8]) -> Result<Self::Public, PublicKeyError> {
        if data.len() != ed25519::PUBLIC_KEY_LENGTH {
            return Err(PublicKeyError::SizeInvalid);
        }
        let mut buf = [0; ed25519::PUBLIC_KEY_LENGTH];
        buf[0..ed25519::PUBLIC_KEY_LENGTH].clone_from_slice(data);
        Ok(Pub(buf))
    }
}

impl VerificationAlgorithm for Ed25519 {
    type Signature = Sig;

    fn signature_from_bytes(data: &[u8]) -> Result<Self::Signature, SignatureError> {
        if data.len() == ed25519::SIGNATURE_LENGTH {
            return Err(SignatureError::SizeInvalid);
        }
        let mut buf = [0; ed25519::SIGNATURE_LENGTH];
        buf[0..ed25519::SIGNATURE_LENGTH].clone_from_slice(data);
        Ok(Sig(buf))
    }

    fn verify_bytes(
        pubkey: &Self::Public,
        signature: &Self::Signature,
        msg: &[u8],
    ) -> Verification {
        ed25519::verify(msg, &pubkey.0, signature.as_ref()).into()
    }
}

impl SigningAlgorithm for Ed25519 {
    fn sign(key: &Self::Secret, msg: &[u8]) -> Sig {
        let (sk, _) = ed25519::keypair(&key.0);
        Sig(ed25519::signature(msg, &sk))
    }
}
