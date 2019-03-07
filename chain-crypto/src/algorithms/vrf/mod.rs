mod dleq;
pub mod vrf;

use crate::key::{AsymmetricKey, PublicKeyError, SecretKeyError};
use rand_core::{CryptoRng, RngCore};

/// VRF
pub struct Curve25519_2HashDH;

impl AsymmetricKey for Curve25519_2HashDH {
    type Secret = vrf::SecretKey;
    type Public = vrf::PublicKey;

    const SECRET_BECH32_HRP: &'static str = "curve25519_2hashdh_secret";
    const PUBLIC_BECH32_HRP: &'static str = "curve25519_2hashdh_public";

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
