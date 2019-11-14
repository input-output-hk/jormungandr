///! Implementation of a sparse array storing maximum of 256 elements

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

mod bitmap;
mod fast;
mod sparse_array;

pub use crate::{
    fast::{FastSparseArray, FastSparseArrayBuilder, FastSparseArrayIter},
    sparse_array::{SparseArray, SparseArrayBuilder, SparseArrayIter},
};
