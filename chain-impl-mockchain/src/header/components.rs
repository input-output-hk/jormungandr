use super::cstruct;
use crate::key::Hash;
use chain_crypto::algorithms::vrf::ProvenOutputSeed;
use chain_crypto::{Ed25519, PublicKey, Signature, SumEd25519_12, Verification};
use std::fmt::{self, Debug};

pub type HeaderId = Hash; // TODO: change to DigestOf<Blake2b256, Header>

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainLength(pub(crate) u32);

impl From<u32> for ChainLength {
    fn from(n: u32) -> ChainLength {
        ChainLength(n)
    }
}

impl From<ChainLength> for u32 {
    fn from(chain_length: ChainLength) -> u32 {
        chain_length.0
    }
}

impl ChainLength {
    pub fn increase(&self) -> Self {
        ChainLength(self.0.checked_add(1).unwrap())
    }
}

impl std::fmt::Display for ChainLength {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct HeaderAuth;

#[derive(Debug, Clone)]
pub struct KESSignature(pub(crate) Signature<HeaderAuth, SumEd25519_12>);

impl From<cstruct::GpKesSignature> for KESSignature {
    fn from(b: cstruct::GpKesSignature) -> KESSignature {
        KESSignature(
            Signature::from_binary(&b[..]).expect("internal error: KES signature length invalid"),
        )
    }
}

impl KESSignature {
    pub fn verify(&self, pk: &PublicKey<SumEd25519_12>, data: &[u8]) -> Verification {
        self.0.verify_slice(pk, data)
    }
}

#[derive(Debug, Clone)]
pub struct BftSignature(pub(crate) Signature<HeaderAuth, Ed25519>);

impl From<cstruct::BftSignature> for BftSignature {
    fn from(b: cstruct::BftSignature) -> BftSignature {
        BftSignature(
            Signature::from_binary(&b[..]).expect("internal error: BFT signature length invalid"),
        )
    }
}

#[derive(Clone)]
pub struct VrfProof(pub(super) cstruct::GpVrfProof);

impl Debug for VrfProof {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("VrfProof")
            .field("data", &&self.0[..])
            .finish()
    }
}

impl VrfProof {
    pub fn to_vrf_proof(&self) -> Option<ProvenOutputSeed> {
        ProvenOutputSeed::from_bytes_unverified(&self.0)
    }
}

impl From<ProvenOutputSeed> for VrfProof {
    fn from(v: ProvenOutputSeed) -> VrfProof {
        VrfProof(v.bytes())
    }
}
