#[cfg(test)]
#[macro_use(lazy_static)]
extern crate lazy_static;

pub mod context;
#[cfg(test)]
pub mod jcli;
#[cfg(test)]
pub mod jormungandr;
#[cfg(all(test, feature = "network"))]
pub mod networking;
#[cfg(all(test, feature = "non-functional"))]
pub mod non_functional;
pub mod startup;
