use crate::certificate::CertificateSlice;
use crate::transaction as tx;
use crate::value::Value;

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

pub trait FeeAlgorithm {
    fn baseline(&self) -> Value;
    fn fees_for_inputs_outputs(&self, inputs: u8, outputs: u8) -> Value;
    fn fees_for_certificate<'a>(&self, cert: CertificateSlice<'a>) -> Value;

    fn calculate<'a>(&self, cert: Option<CertificateSlice<'a>>, inputs: u8, outputs: u8) -> Value {
        self.baseline()
            .saturating_add(self.fees_for_inputs_outputs(inputs, outputs))
            .saturating_add(cert.map_or(Value::zero(), |c| self.fees_for_certificate(c)))
    }

    fn calculate_tx<P: tx::Payload>(&self, tx: &tx::Transaction<P>) -> Value {
        self.calculate(tx.as_slice().payload().to_certificate_slice(), tx.nb_inputs(), tx.nb_outputs())
    }

}

impl FeeAlgorithm for LinearFee {
    fn baseline(&self) -> Value {
        Value(self.constant)
    }
    fn fees_for_inputs_outputs(&self, inputs: u8, outputs: u8) -> Value {
        Value(self.coefficient.saturating_mul((inputs as u64) + (outputs as u64)))
    }
    fn fees_for_certificate<'a>(&self, _: CertificateSlice<'a>) -> Value {
        Value(self.certificate)
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
