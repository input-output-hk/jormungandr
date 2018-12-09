use super::super::super::secure::crypto::sign::SignatureAlgorithm;
use super::identity::{StakerIdentity, StakerSignature};

pub struct Certificate {
    delegatee: StakerIdentity,
    delegator: StakerIdentity,
    signature: StakerSignature,
}
