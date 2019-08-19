use crate::certificate::Certificate;
use crate::transaction as tx;
use crate::value::Value;
use chain_addr::Address;

/// Linear fee using the basic affine formula
/// `COEFFICIENT * bytes(COUNT(tx.inputs) + COUNT(tx.outputs)) + CONSTANT + CERTIFICATE*COUNT(certificates)`.
#[derive(PartialEq, Eq, PartialOrd, Debug, Clone, Copy)]
pub struct LinearFee {
    pub constant: u64,
    pub coefficient: u64,
    pub certificate: u64,
}

impl LinearFee {
    pub fn new(constant: u64, coefficient: u64, certificate: u64) -> Self {
        LinearFee {
            constant,
            coefficient,
            certificate,
        }
    }
}

pub trait FeeAlgorithm<P> {
    fn calculate(&self, part: &P) -> Option<Value>;
}

impl<'a, P, FA: FeeAlgorithm<P>> FeeAlgorithm<P> for &'a FA {
    fn calculate(&self, part: &P) -> Option<Value> {
        (*self).calculate(part)
    }
}

impl FeeAlgorithm<tx::Transaction<Address, tx::NoExtra>> for LinearFee {
    fn calculate(&self, tx: &tx::Transaction<Address, tx::NoExtra>) -> Option<Value> {
        let msz = (tx.inputs.len() as u64).checked_add(tx.outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?;
        Some(Value(fee))
    }
}

impl FeeAlgorithm<tx::Transaction<Address, Certificate>> for LinearFee {
    fn calculate(&self, tx: &tx::Transaction<Address, Certificate>) -> Option<Value> {
        let msz = (tx.inputs.len() as u64).checked_add(tx.outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?
            .checked_add(self.certificate)?;
        Some(Value(fee))
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for LinearFee {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                constant: Arbitrary::arbitrary(g),
                coefficient: Arbitrary::arbitrary(g),
                certificate: Arbitrary::arbitrary(g),
            }
        }
    }
}
