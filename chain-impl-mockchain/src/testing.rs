use crate::transaction::Output;
use crate::value::Value;
use chain_addr::Address;
use quickcheck::{Arbitrary, Gen};

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
