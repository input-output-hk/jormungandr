#![cfg(test)]

pub mod common;
pub mod jcli;
pub mod jormungandr;
pub mod networking;
pub mod non_functional;

// The purpose of this file is to allow cargo correctly detect tests located in subfolders. It acts like lib.rs file.
