#![allow(dead_code)]
use jormungandr_lib::{crypto::hash::Hash, interfaces::Block0Configuration};

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum EitherHashOrBlock0 {
    Hash(Hash),
    Block0(Block0Configuration),
}
