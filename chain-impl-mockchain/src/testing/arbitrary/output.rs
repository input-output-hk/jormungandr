use crate::transaction::Output;
use chain_addr::{Address, Discrimination, Kind};
use quickcheck::{Arbitrary, Gen};
use std::iter;

#[derive(Clone, Debug)]
pub struct OutputsWithoutMultisig(pub Vec<Output<Address>>);

impl Arbitrary for OutputsWithoutMultisig {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let n = usize::arbitrary(gen);
        OutputsWithoutMultisig(
            iter::from_fn(|| Some(Output::arbitrary(gen)))
                .filter(|x| match x.address.1 {
                    Kind::Multisig { .. } => false,
                    _ => true,
                })
                .take(n)
                .collect(),
        )
    }
}

impl OutputsWithoutMultisig {
    pub fn set_discrimination(&mut self, discrimination: Discrimination) {
        for output in &mut self.0 {
            output.address.0 = discrimination.clone();
        }
    }
}
