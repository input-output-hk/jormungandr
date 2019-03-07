#![cfg_attr(feature = "with-bench", feature(test))]

#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

extern crate cryptoxide;
extern crate ed25519_bip32;
extern crate rand_core;

pub mod algorithms;
pub mod hash;
mod hex;
mod kes;
mod key;
mod sign;

pub use kes::KeyEvolvingSignatureAlgorithm;
pub use key::{AsymmetricKey, KeyPair, PublicKey, PublicKeyError, SecretKey, SecretKeyError};
pub use sign::{Signature, SignatureError, SigningAlgorithm, Verification, VerificationAlgorithm};

pub use algorithms::*;
pub use hash::Blake2b256;
