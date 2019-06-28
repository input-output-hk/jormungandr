pub mod address;
pub mod kind_type;
pub mod output;

use quickcheck::{Arbitrary, Gen};
use crate::value::Value;
use crate::transaction::Output;
use chain_addr::Address;

pub use address::*;
pub use kind_type::*;
pub use output::*;

impl Arbitrary for Value {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        Value(u64::arbitrary(gen))
    }
}

impl Arbitrary for Output<Address> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Output {
            address: Arbitrary::arbitrary(g),
            value: Arbitrary::arbitrary(g),
        }
    }
}