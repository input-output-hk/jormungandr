mod dleq;
pub mod vrf;

use crate::key::{
    AsymmetricKey, AsymmetricPublicKey, PublicKeyError, SecretKeyError, SecretKeySizeStatic,
};
use crate::vrf::{VRFVerification, VerifiableRandomFunction};
use rand_core::{CryptoRng, RngCore};

/// VRF
pub struct Curve25519_2HashDH;

impl AsymmetricPublicKey for Curve25519_2HashDH {
    type Public = vrf::PublicKey;
    const PUBLIC_BECH32_HRP: &'static str = "vrf_pk";
    const PUBLIC_KEY_SIZE: usize = vrf::PUBLIC_SIZE;
    fn public_from_binary(data: &[u8]) -> Result<Self::Public, PublicKeyError> {
        vrf::PublicKey::from_bytes(data)
    }
}

impl AsymmetricKey for Curve25519_2HashDH {
    type Secret = vrf::SecretKey;
    type PubAlg = Curve25519_2HashDH;

    const SECRET_BECH32_HRP: &'static str = "vrf_sk";

    fn generate<T: RngCore + CryptoRng>(rng: T) -> Self::Secret {
        Self::Secret::random(rng)
    }

    fn compute_public(key: &Self::Secret) -> <Self::PubAlg as AsymmetricPublicKey>::Public {
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
}

impl SecretKeySizeStatic for Curve25519_2HashDH {
    const SECRET_KEY_SIZE: usize = vrf::SECRET_SIZE;
}

impl VerifiableRandomFunction for Curve25519_2HashDH {
    type VerifiedRandomOutput = vrf::ProvenOutputSeed;
    type RandomOutput = vrf::OutputSeed;
    type Input = [u8];

    const VERIFIED_RANDOM_SIZE: usize = vrf::PROOF_SIZE;

    fn evaluate_and_prove<T: RngCore + CryptoRng>(
        secret: &Self::Secret,
        input: &Self::Input,
        mut rng: T,
    ) -> Self::VerifiedRandomOutput {
        secret.evaluate_simple(&mut rng, input)
    }

    fn verify(
        public: &Self::Public,
        input: &Self::Input,
        vrand: &Self::VerifiedRandomOutput,
    ) -> VRFVerification {
        let v = vrand.verify(public, input);
        if v {
            VRFVerification::Success
        } else {
            VRFVerification::Failed
        }
    }

    fn strip_verification_output(vr: &Self::VerifiedRandomOutput) -> Self::RandomOutput {
        vr.u.clone()
    }
}
