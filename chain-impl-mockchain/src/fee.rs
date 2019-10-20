use crate::certificate::{
    Certificate, OwnerStakeDelegation, PoolManagement, PoolRegistration, StakeDelegation,
};
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
    fn calculate(&self, part: &P, inputs: &[tx::Input], output: &[tx::Output<Address>]) -> Option<Value>;
}

impl<'a, P, FA: FeeAlgorithm<P>> FeeAlgorithm<P> for &'a FA {
    fn calculate(&self, part: &P, inputs: &[tx::Input], outputs: &[tx::Output<Address>]) -> Option<Value> {
        (*self).calculate(part, inputs, outputs)
    }
}

impl FeeAlgorithm<tx::NoExtra> for LinearFee {
    fn calculate(&self, p: &tx::NoExtra, inputs: &[tx::Input], outputs: &[tx::Output<Address>]) -> Option<Value> {
        let msz = (inputs.len() as u64).checked_add(outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?;
        Some(Value(fee))
    }
}

impl FeeAlgorithm<PoolRegistration> for LinearFee {
    fn calculate(&self, _: &PoolRegistration, inputs: &[tx::Input], outputs: &[tx::Output<Address>]) -> Option<Value> {
        let msz = (inputs.len() as u64).checked_add(outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?
            .checked_add(self.certificate)?;
        Some(Value(fee))
    }
}

impl FeeAlgorithm<PoolManagement> for LinearFee {
    fn calculate(&self, _: &PoolManagement, inputs: &[tx::Input], outputs: &[tx::Output<Address>]) -> Option<Value> {
        let msz = (inputs.len() as u64).checked_add(outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?
            .checked_add(self.certificate)?;
        Some(Value(fee))
    }
}

impl FeeAlgorithm<OwnerStakeDelegation> for LinearFee {
    fn calculate(&self, _: &OwnerStakeDelegation, inputs: &[tx::Input], outputs: &[tx::Output<Address>]) -> Option<Value> {
        let msz = (inputs.len() as u64).checked_add(outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?
            .checked_add(self.certificate)?;
        Some(Value(fee))
    }
}

impl FeeAlgorithm<StakeDelegation> for LinearFee {
    fn calculate(&self, _: &StakeDelegation, inputs: &[tx::Input], outputs: &[tx::Output<Address>]) -> Option<Value> {
        let msz = (inputs.len() as u64).checked_add(outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?
            .checked_add(self.certificate)?;
        Some(Value(fee))
    }
}

impl FeeAlgorithm<Certificate> for LinearFee {
    fn calculate(&self, cert: &Certificate, inputs: &[tx::Input], outputs: &[tx::Output<Address>]) -> Option<Value> {
        match cert {
            Certificate::PoolManagement(c) => self.calculate(c, inputs, outputs),
            Certificate::PoolRegistration(c) => self.calculate(c, inputs, outputs),
            Certificate::StakeDelegation(c) => self.calculate(c, inputs, outputs),
            Certificate::OwnerStakeDelegation(c) => self.calculate(c, inputs, outputs),
        }
    }
}

impl FeeAlgorithm<Option<Certificate>> for LinearFee {
    fn calculate(&self, cert: &Option<Certificate>, inputs: &[tx::Input], outputs: &[tx::Output<Address>]) -> Option<Value> {
        match cert {
            None => self.calculate(&tx::NoExtra, inputs, outputs),
            Some(c) => self.calculate(c, inputs, outputs),
        }
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
