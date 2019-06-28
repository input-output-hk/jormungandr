use chain_addr::Address;
use crate::transaction::Output;
use crate::value::Value;
use quickcheck::{Arbitrary, Gen};

pub mod common;
pub mod genesis;
pub mod ledger;

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