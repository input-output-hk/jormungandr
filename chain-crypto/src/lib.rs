#![cfg_attr(feature = "with-bench", feature(test))]

#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

extern crate cryptoxide;
extern crate rand_core;

pub mod algorithms;
mod hex;
mod kes;
mod key;
mod sign;

pub use key::{AsymmetricKey, PublicKey, PublicKeyError, SecretKey, SecretKeyError};
pub use sign::Signature;

pub use algorithms::*;
