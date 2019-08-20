#[macro_use(lazy_static)]
extern crate lazy_static;
#[macro_use(error_chain, bail)]
extern crate error_chain;

#[cfg(test)]
pub mod common;
#[cfg(test)]
pub mod jcli;
#[cfg(test)]
pub mod jormungandr;
#[cfg(test)]
pub mod networking;
#[cfg(test)]
pub mod non_functional;

pub mod v2;
