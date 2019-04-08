#![cfg_attr(feature = "with-bench", feature(test))]

#[macro_use]
extern crate cfg_if;

#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

cfg_if! {
    if #[cfg(test)] {
        mod testing;
    } else if #[cfg(feature = "property-test-api")] {
        mod testing;
    }
}

pub mod algorithms;
pub mod bech32;
pub mod hash;
mod hex;
mod kes;
mod key;
mod sign;
mod vrf;

pub use kes::KeyEvolvingSignatureAlgorithm;
pub use key::{AsymmetricKey, KeyPair, PublicKey, PublicKeyError, SecretKey, SecretKeyError};
pub use sign::{Signature, SignatureError, SigningAlgorithm, Verification, VerificationAlgorithm};
pub use vrf::{
    vrf_evaluate_and_proove, vrf_verified_get_output, vrf_verify, VRFVerification,
    VerifiableRandomFunction,
};

pub use algorithms::*;
pub use hash::{Blake2b224, Blake2b256, Sha3_256};
