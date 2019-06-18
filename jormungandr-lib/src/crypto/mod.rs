//! This module provides wrapper around the different cryptographic
//! objects used in jormungandr, the jcli and the tests
//!
//! # Hash
//!
//! The [`Hash`] is a Blake2b256. It is used to identify a block,
//! a transaction, a fragment, a Node Id. The type used here provides
//! a consistent API for user interaction with a Hash. Which ever be
//! its purpose.
//!
//! # Signing and Identifier keys
//!
//! This is a very generic type wrapper around `chain-crypto` secret
//! and public keys. It provides the appropriate serialization format
//! for human readable interfaces depending on the purpose.
//!
//! In a human readable serializer for `serde` (like `serde_yaml` or
//! `serde_json`) it will give a bech32 encoding. But utilising
//! `Display` will provide an hexadecimal encoding version of the key.
//!
//! # Account keys
//!
//! The proper type for the account management and interfaces.
//! It provides the same interfaces as for the identifier in the
//! `key` module but limited to Account only.
//!

pub mod account;
pub mod hash;
pub mod key;
pub(crate) mod serde;
