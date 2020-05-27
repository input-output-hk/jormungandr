#[cfg(test)]
#[macro_use(lazy_static)]
extern crate lazy_static;

#[macro_use]
extern crate slog;

#[cfg(test)]
pub mod jcli;
#[cfg(test)]
pub mod jormungandr;
#[cfg(test)]
pub mod networking;
#[cfg(test)]
pub mod non_functional;

pub mod common;
pub mod mock;
