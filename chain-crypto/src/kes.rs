use crate::sign::VerificationAlgorithm;

pub trait KeyEvolvingSignatureAlgorithm: VerificationAlgorithm {
    /// Get the period associated with this signature
    fn get_period(key: &Self::Signature) -> usize;

    /// Update the key to the next period
    ///
    /// if false is returned, then the key couldn't be updated
    fn update(key: &mut Self::Secret) -> bool;

    /// Sign with the current secret key and update to the next period
    fn sign_update(key: &mut Self::Secret, msg: &[u8]) -> Self::Signature;
}
