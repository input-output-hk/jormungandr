use crate::kes::KeyEvolvingSignatureAlgorithm;
use crate::key::{AsymmetricKey, PublicKeyError, SecretKeyError};
use crate::sign::{SignatureError, Verification, VerificationAlgorithm};
use cryptoxide::ed25519;
use rand_core::{CryptoRng, RngCore};

/// Fake MMM Signing Algorithm
pub struct FakeMMM;

#[derive(Clone)]
pub struct Priv([u8; ed25519::PRIVATE_KEY_LENGTH]);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Pub([u8; ed25519::PUBLIC_KEY_LENGTH]);

#[derive(Clone)]
pub struct Sig([u8; ed25519::SIGNATURE_LENGTH]);

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

impl AsymmetricKey for FakeMMM {
    type Secret = Priv;
    type Public = Pub;

    const SECRET_BECH32_HRP: &'static str = "fakemmm_secret";
    const PUBLIC_BECH32_HRP: &'static str = "fakemmm_public";

    fn generate<T: RngCore + CryptoRng>(mut rng: T) -> Priv {
        let mut priv_bytes = [0u8; ed25519::PRIVATE_KEY_LENGTH];
        rng.fill_bytes(&mut priv_bytes);
        Priv(priv_bytes)
    }

    fn compute_public(key: &Priv) -> Pub {
        let (_, pk) = ed25519::keypair(&key.0);
        Pub(pk)
    }

    fn secret_from_binary(data: &[u8]) -> Result<Priv, SecretKeyError> {
        if data.len() != ed25519::PRIVATE_KEY_LENGTH {
            return Err(SecretKeyError::SizeInvalid);
        }
        let mut buf = [0; ed25519::PRIVATE_KEY_LENGTH];
        buf[0..ed25519::PRIVATE_KEY_LENGTH].clone_from_slice(data);
        Ok(Priv(buf))
    }
    fn public_from_binary(data: &[u8]) -> Result<Pub, PublicKeyError> {
        if data.len() != ed25519::PUBLIC_KEY_LENGTH {
            return Err(PublicKeyError::SizeInvalid);
        }
        let mut buf = [0; ed25519::PUBLIC_KEY_LENGTH];
        buf[0..ed25519::PUBLIC_KEY_LENGTH].clone_from_slice(data);
        Ok(Pub(buf))
    }
}

impl VerificationAlgorithm for FakeMMM {
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

impl KeyEvolvingSignatureAlgorithm for FakeMMM {
    fn sign_update(key: &mut Self::Secret, msg: &[u8]) -> Sig {
        let (sk, _) = ed25519::keypair(&key.0);
        Sig(ed25519::signature(msg, &sk))
    }
}
