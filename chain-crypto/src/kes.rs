use crate::sign::VerificationAlgorithm;

pub trait KeyEvolvingSignatureAlgorithm: VerificationAlgorithm {
    fn sign_update(key: &mut Self::Secret, msg: &[u8]) -> Self::Signature;
}
