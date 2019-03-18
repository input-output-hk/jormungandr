mod dleq;
pub mod vrf;

use crate::key::{AsymmetricKey, PublicKeyError, SecretKeyError};
use crate::vrf::{VerifiableRandomFunction, Verification};
use rand::{CryptoRng, RngCore};

/// VRF
pub struct Curve25519_2HashDH;

impl AsymmetricKey for Curve25519_2HashDH {
    type Secret = vrf::SecretKey;
    type Public = vrf::PublicKey;

    const SECRET_BECH32_HRP: &'static str = "curve25519_2hashdh_secret";
    const PUBLIC_BECH32_HRP: &'static str = "curve25519_2hashdh_public";

    const SECRET_KEY_SIZE: usize = vrf::SECRET_SIZE;
    const PUBLIC_KEY_SIZE: usize = vrf::PUBLIC_SIZE;

    fn generate<T: RngCore + CryptoRng>(rng: T) -> Self::Secret {
        Self::Secret::random(rng)
    }

    fn compute_public(key: &Self::Secret) -> Self::Public {
        key.public()
    }

    fn secret_from_binary(data: &[u8]) -> Result<Self::Secret, SecretKeyError> {
        if data.len() != vrf::SECRET_SIZE {
            return Err(SecretKeyError::SizeInvalid);
        }
        let mut buf = [0; vrf::SECRET_SIZE];
        buf[0..vrf::SECRET_SIZE].clone_from_slice(data);
        match vrf::SecretKey::from_bytes(buf) {
            None => Err(SecretKeyError::StructureInvalid),
            Some(k) => Ok(k),
        }
    }
    fn public_from_binary(data: &[u8]) -> Result<Self::Public, PublicKeyError> {
        vrf::PublicKey::from_bytes(data)
    }
}

impl VerifiableRandomFunction for Curve25519_2HashDH {
    type VerifiedRandom = vrf::ProvenOutputSeed;
    type Input = [u8];

    fn evaluate<T: RngCore + CryptoRng>(
        secret: &Self::Secret,
        input: &Self::Input,
        mut rng: T,
    ) -> Self::VerifiedRandom {
        secret.evaluate_simple(&mut rng, input)
    }

    fn verify(
        public: &Self::Public,
        input: &Self::Input,
        vrand: &Self::VerifiedRandom,
    ) -> Verification {
        let v = vrand.verify(public, input);
        if v {
            Verification::Success
        } else {
            Verification::Failed
        }
    }
}
