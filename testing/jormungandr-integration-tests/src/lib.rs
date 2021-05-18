#[cfg(test)]
#[macro_use(lazy_static)]
extern crate lazy_static;

#[cfg(test)]
pub mod jcli;
#[cfg(test)]
pub mod jormungandr;
#[cfg(test)]
pub mod networking;
#[cfg(test)]
pub mod non_functional;
#[cfg(test)]
pub mod reordered_fragments;

pub mod common;
