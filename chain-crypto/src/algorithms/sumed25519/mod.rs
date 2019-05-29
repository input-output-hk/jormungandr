mod common;
mod sum;

#[cfg(test)]
mod sumrec;

use crate::evolving::{EvolvingStatus, KeyEvolvingAlgorithm};
use crate::kes::KeyEvolvingSignatureAlgorithm;
use crate::key::{AsymmetricKey, PublicKeyError, SecretKeyError};
use crate::sign::{SignatureError, SigningAlgorithm, Verification, VerificationAlgorithm};
use rand::{CryptoRng, RngCore};

// MMM sum scheme instanciated over the Ed25519 signature system
// and a specific depth of 12
pub struct SumEd25519_12;

const DEPTH: common::Depth = common::Depth(12);

impl AsymmetricKey for SumEd25519_12 {
    type Secret = sum::SecretKey;
    type Public = sum::PublicKey;

    const SECRET_BECH32_HRP: &'static str = "kes25519-12-sk";
    const PUBLIC_BECH32_HRP: &'static str = "kes25519-12-pk";

    const PUBLIC_KEY_SIZE: usize = 32;

    fn generate<T: RngCore + CryptoRng>(mut rng: T) -> Self::Secret {
        let mut priv_bytes = [0u8; common::Seed::SIZE];
        rng.fill_bytes(&mut priv_bytes);

        let seed = common::Seed::from_bytes(priv_bytes);

        let (sk, _) = sum::keygen(DEPTH, &seed);
        sk
    }

    fn compute_public(key: &Self::Secret) -> Self::Public {
        key.compute_public()
    }

    fn secret_from_binary(data: &[u8]) -> Result<Self::Secret, SecretKeyError> {
        sum::SecretKey::from_bytes(DEPTH, data).map_err(|e| match e {
            sum::Error::InvalidSecretKeySize(_) => SecretKeyError::SizeInvalid,
            _ => SecretKeyError::StructureInvalid,
        })
    }
    fn public_from_binary(data: &[u8]) -> Result<Self::Public, PublicKeyError> {
        sum::PublicKey::from_bytes(data).map_err(|e| match e {
            sum::Error::InvalidPublicKeySize(_) => PublicKeyError::SizeInvalid,
            _ => PublicKeyError::StructureInvalid,
        })
    }
}

impl VerificationAlgorithm for SumEd25519_12 {
    type Signature = sum::Signature;

    const SIGNATURE_SIZE: usize = sum::signature_size(DEPTH);
    const SIGNATURE_BECH32_HRP: &'static str = "kes25519-12-sig";

    fn signature_from_bytes(data: &[u8]) -> Result<Self::Signature, SignatureError> {
        sum::Signature::from_bytes(DEPTH, data).map_err(|e| match e {
            sum::Error::InvalidSignatureSize(_) => SignatureError::SizeInvalid {
                expected: Self::SIGNATURE_SIZE,
                got: data.len(),
            },
            _ => SignatureError::StructureInvalid,
        })
    }

    fn verify_bytes(
        pubkey: &Self::Public,
        signature: &Self::Signature,
        msg: &[u8],
    ) -> Verification {
        if sum::verify(pubkey, msg, signature) {
            Verification::Success
        } else {
            Verification::Failed
        }
    }
}

impl SigningAlgorithm for SumEd25519_12 {
    fn sign(key: &Self::Secret, msg: &[u8]) -> Self::Signature {
        sum::sign(key, msg)
    }
}

impl KeyEvolvingAlgorithm for SumEd25519_12 {
    fn get_period(sec: &Self::Secret) -> usize {
        sec.t()
    }
    fn update(key: &mut Self::Secret) -> EvolvingStatus {
        if sum::update(key).is_ok() {
            EvolvingStatus::Success
        } else {
            EvolvingStatus::Failed
        }
    }
}

impl KeyEvolvingSignatureAlgorithm for SumEd25519_12 {
    fn get_period(sig: &Self::Signature) -> usize {
        sig.t()
    }
}
