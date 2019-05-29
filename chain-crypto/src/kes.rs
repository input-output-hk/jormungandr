use crate::sign::VerificationAlgorithm;

pub trait KeyEvolvingSignatureAlgorithm: VerificationAlgorithm {
    /// Get the period associated with this signature
    fn get_period(sig: &Self::Signature) -> usize;
}
