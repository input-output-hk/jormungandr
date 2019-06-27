use super::KindWithoutMultisig;
use chain_addr::{Address, Discrimination};
use chain_impl_mockchain::transaction::Output;
use quickcheck::{Arbitrary, Gen};

#[derive(Clone, Debug)]
pub struct OutputsWithoutMultisig(pub Vec<Output<Address>>);

impl Arbitrary for OutputsWithoutMultisig {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let count = u8::arbitrary(gen);
        let mut outputs = vec![];
        for _ in 0..count {
            let mut output = Output::arbitrary(gen);
            output.address.1 = KindWithoutMultisig::arbitrary(gen).0;
            outputs.push(output);
        }
        OutputsWithoutMultisig(outputs)
    }
}

impl OutputsWithoutMultisig {
    pub fn set_discrimination(&mut self, discrimination: Discrimination) {
        for output in &mut self.0 {
            output.address.0 = discrimination.clone();
        }
    }
}
