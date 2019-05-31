use crate::key;
use rand::{CryptoRng, RngCore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VRFVerification {
    Success,
    Failed,
}

pub trait VerifiableRandomFunction: key::AsymmetricPublicKey + key::AsymmetricKey {
    type VerifiedRandomOutput;
    type RandomOutput;
    type Input: ?Sized;

    const VERIFIED_RANDOM_SIZE: usize;

    fn evaluate_and_prove<T: RngCore + CryptoRng>(
        secret: &Self::Secret,
        input: &Self::Input,
        rng: T,
    ) -> Self::VerifiedRandomOutput;

    fn verify(
        public: &Self::Public,
        input: &Self::Input,
        vrand: &Self::VerifiedRandomOutput,
    ) -> VRFVerification;

    fn strip_verification_output(vr: &Self::VerifiedRandomOutput) -> Self::RandomOutput;
}

/// Evaluate the VRF for a specific input and return a verified output
pub fn vrf_evaluate_and_prove<VRF: VerifiableRandomFunction, T: RngCore + CryptoRng>(
    secret: &key::SecretKey<VRF>,
    input: &<VRF as VerifiableRandomFunction>::Input,
    rng: T,
) -> <VRF as VerifiableRandomFunction>::VerifiedRandomOutput {
    VRF::evaluate_and_prove(&secret.0, input, rng)
}

/// Verify the VRF output for a specific input is correct
pub fn vrf_verify<VRF: VerifiableRandomFunction>(
    public: &key::PublicKey<VRF>,
    input: &<VRF as VerifiableRandomFunction>::Input,
    vrand: &<VRF as VerifiableRandomFunction>::VerifiedRandomOutput,
) -> VRFVerification {
    VRF::verify(&public.0, input, vrand)
}

pub fn vrf_verified_get_output<VRF: VerifiableRandomFunction>(
    vr: &<VRF as VerifiableRandomFunction>::VerifiedRandomOutput,
) -> <VRF as VerifiableRandomFunction>::RandomOutput {
    VRF::strip_verification_output(vr)
}
