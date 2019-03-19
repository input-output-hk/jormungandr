use crate::key;
use rand::{CryptoRng, RngCore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verification {
    Success,
    Failed,
}

pub trait VerifiableRandomFunction: key::AsymmetricKey {
    type VerifiedRandom;
    type Input: ?Sized;

    const VERIFIED_RANDOM_SIZE: usize;

    fn evaluate<T: RngCore + CryptoRng>(
        secret: &Self::Secret,
        input: &Self::Input,
        rng: T,
    ) -> Self::VerifiedRandom;

    fn verify(
        public: &Self::Public,
        input: &Self::Input,
        vrand: &Self::VerifiedRandom,
    ) -> Verification;
}

/// Evaluate the VRF for a specific input
pub fn vrf_evaluate<VRF: VerifiableRandomFunction, T: RngCore + CryptoRng>(
    secret: &key::SecretKey<VRF>,
    input: &<VRF as VerifiableRandomFunction>::Input,
    rng: T,
) -> <VRF as VerifiableRandomFunction>::VerifiedRandom {
    VRF::evaluate(&secret.0, input, rng)
}

/// Verify the VRF output for a specific input is correct
pub fn vrf_verify<VRF: VerifiableRandomFunction>(
    public: &key::PublicKey<VRF>,
    input: &<VRF as VerifiableRandomFunction>::Input,
    vrand: &<VRF as VerifiableRandomFunction>::VerifiedRandom,
) -> Verification {
    VRF::verify(&public.0, input, vrand)
}
