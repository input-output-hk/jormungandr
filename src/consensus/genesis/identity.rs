//use cryptoxide::ed25519;
use super::super::super::secure::crypto::sign::{self, SignatureAlgorithm};

/// Identity of a staker
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StakerIdentity(<sign::Ed25519 as SignatureAlgorithm>::PublicKey);

/// Staker Secret
pub struct StakerSecret(<sign::Ed25519 as SignatureAlgorithm>::SecretKey);

/// Staker Signature
pub struct StakerSignature(<sign::Ed25519 as SignatureAlgorithm>::Signature);

//impl StakerSecret {
//}
