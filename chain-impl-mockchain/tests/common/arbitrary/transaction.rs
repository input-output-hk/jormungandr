use crate::common::address::AddressData;
use chain_addr::Address;
use chain_impl_mockchain::transaction::Output;
use chain_impl_mockchain::value::Value;
use quickcheck::{Arbitrary, Gen};

#[derive(Clone, Debug)]
pub struct ArbitraryValue(pub Value);

impl Arbitrary for ArbitraryValue {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        ArbitraryValue(Value(u64::arbitrary(gen)))
    }
}

#[derive(Clone, Debug)]
pub struct ArbitraryOutput(pub Output<Address>);

impl Arbitrary for ArbitraryOutput {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let value = ArbitraryValue::arbitrary(gen);
        let address = AddressData::arbitrary(gen);
        ArbitraryOutput(address.as_output(value.0))
    }
}

#[derive(Clone, Debug)]
pub struct ArbitraryOutputs(pub Vec<ArbitraryOutput>);

impl ArbitraryOutputs {
    pub fn outputs(&self) -> Vec<Output<Address>> {
        self.0.iter().map(|x| x.0.clone()).collect()
    }
}

impl Arbitrary for ArbitraryOutputs {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let count = u8::arbitrary(gen);
        let mut outputs = vec![];
        for _ in 0..count {
            outputs.push(ArbitraryOutput::arbitrary(gen));
        }
        ArbitraryOutputs(outputs)
    }
}
