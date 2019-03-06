use crate::key::{AsymmetricKey, PublicKeyError, SecretKeyError};
use crate::sign::{SignatureError, SigningAlgorithm, Verification, VerificationAlgorithm};

use super::ed25519 as ei;

use cryptoxide::ed25519;
use rand_core::{CryptoRng, RngCore};

/// ED25519 Signing Algorithm with extended secret key
pub struct Ed25519Extended;

#[derive(Clone)]
pub struct ExtendedPriv([u8; 64]);

impl AsRef<[u8]> for ExtendedPriv {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl AsymmetricKey for Ed25519Extended {
    type Secret = ExtendedPriv;
    type Public = ei::Pub;

    fn generate<T: RngCore + CryptoRng>(mut rng: T) -> Self::Secret {
        let mut priv_bytes = [0u8; ed25519::PRIVATE_KEY_LENGTH];
        rng.fill_bytes(&mut priv_bytes);
        let (sk, _) = ed25519::keypair(&priv_bytes);
        ExtendedPriv(sk)
    }

    fn compute_public(key: &Self::Secret) -> Self::Public {
        let pk = ed25519::to_public(&key.0);
        ei::Pub(pk)
    }

    fn secret_from_binary(data: &[u8]) -> Result<Self::Secret, SecretKeyError> {
        if data.len() != ed25519::PRIVATE_KEY_LENGTH {
            return Err(SecretKeyError::SizeInvalid);
        }
        let mut buf = [0; 64];
        buf[0..64].clone_from_slice(data);
        /// TODO structure check
        Ok(ExtendedPriv(buf))
    }
    fn public_from_binary(data: &[u8]) -> Result<Self::Public, PublicKeyError> {
        if data.len() != ed25519::PUBLIC_KEY_LENGTH {
            return Err(PublicKeyError::SizeInvalid);
        }
        let mut buf = [0; ed25519::PUBLIC_KEY_LENGTH];
        buf[0..ed25519::PUBLIC_KEY_LENGTH].clone_from_slice(data);
        Ok(ei::Pub(buf))
    }
}

impl VerificationAlgorithm for Ed25519Extended {
    type Signature = ei::Sig;

    fn signature_from_bytes(data: &[u8]) -> Result<Self::Signature, SignatureError> {
        if data.len() == ed25519::SIGNATURE_LENGTH {
            return Err(SignatureError::SizeInvalid);
        }
        let mut buf = [0; ed25519::SIGNATURE_LENGTH];
        buf[0..ed25519::SIGNATURE_LENGTH].clone_from_slice(data);
        Ok(ei::Sig(buf))
    }

    fn verify_bytes(
        pubkey: &Self::Public,
        signature: &Self::Signature,
        msg: &[u8],
    ) -> Verification {
        ed25519::verify(msg, &pubkey.0, signature.as_ref()).into()
    }
}

impl SigningAlgorithm for Ed25519Extended {
    fn sign(key: &Self::Secret, msg: &[u8]) -> ei::Sig {
        ei::Sig(ed25519::signature_extended(msg, &key.0))
    }
}
