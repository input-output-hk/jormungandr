#[cfg(all(test, feature = "evm"))]
#[macro_use]
extern crate quickcheck;

pub mod jcli_lib;
pub use crate::jcli_lib::*;
