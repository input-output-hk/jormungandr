#![cfg_attr(feature = "with-bench", feature(test))]

#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

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
pub use vrf::{vrf_evaluate, vrf_verify, VerifiableRandomFunction};

pub use algorithms::*;
pub use hash::Blake2b256;
