#![cfg_attr(feature = "with-bench", feature(test))]

#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

mod derivation;
mod hex;
mod key;
mod securemem;
mod signature;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[cfg(feature = "with-bench")]
mod bench;

pub use derivation::{DerivationIndex, DerivationScheme};
pub use key::{PrivateKeyError, PublicKeyError, XPrv, XPub, XPRV_SIZE, XPUB_SIZE};
pub use signature::{Signature, SignatureError, SIGNATURE_SIZE};
