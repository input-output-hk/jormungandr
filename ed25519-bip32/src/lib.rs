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
#[cfg(feature = "with-bench")]
mod bench;

pub use derivation::DerivationScheme;
pub use key::{XPrv, XPub, XPRV_SIZE, XPUB_SIZE};
pub use signature::Signature;
