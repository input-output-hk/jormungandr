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

impl FeeAlgorithm<tx::Transaction<Address, PoolRegistration>> for LinearFee {
    fn calculate(&self, tx: &tx::Transaction<Address, PoolRegistration>) -> Option<Value> {
        let msz = (tx.inputs.len() as u64).checked_add(tx.outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?
            .checked_add(self.certificate)?;
        Some(Value(fee))
    }
}

impl FeeAlgorithm<tx::Transaction<Address, PoolManagement>> for LinearFee {
    fn calculate(&self, tx: &tx::Transaction<Address, PoolManagement>) -> Option<Value> {
        let msz = (tx.inputs.len() as u64).checked_add(tx.outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?
            .checked_add(self.certificate)?;
        Some(Value(fee))
    }
}

impl FeeAlgorithm<tx::Transaction<Address, OwnerStakeDelegation>> for LinearFee {
    fn calculate(&self, tx: &tx::Transaction<Address, OwnerStakeDelegation>) -> Option<Value> {
        let msz = (tx.inputs.len() as u64).checked_add(tx.outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?
            .checked_add(self.certificate)?;
        Some(Value(fee))
    }
}

impl FeeAlgorithm<tx::Transaction<Address, StakeDelegation>> for LinearFee {
    fn calculate(&self, tx: &tx::Transaction<Address, StakeDelegation>) -> Option<Value> {
        let msz = (tx.inputs.len() as u64).checked_add(tx.outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?
            .checked_add(self.certificate)?;
        Some(Value(fee))
    }
}

impl FeeAlgorithm<tx::Transaction<Address, Certificate>> for LinearFee {
    fn calculate(&self, tx: &tx::Transaction<Address, Certificate>) -> Option<Value> {
        match &tx.extra {
            Certificate::PoolManagement(c) => self.calculate(&tx.clone().replace_extra(c.clone())),
            Certificate::PoolRegistration(c) => {
                self.calculate(&tx.clone().replace_extra(c.clone()))
            }
            Certificate::StakeDelegation(c) => self.calculate(&tx.clone().replace_extra(c.clone())),
            Certificate::OwnerStakeDelegation(c) => {
                self.calculate(&tx.clone().replace_extra(c.clone()))
            }
        }
    }
}

impl FeeAlgorithm<tx::Transaction<Address, Option<Certificate>>> for LinearFee {
    fn calculate(&self, tx: &tx::Transaction<Address, Option<Certificate>>) -> Option<Value> {
        match &tx.extra {
            None => self.calculate(&tx.clone().replace_extra(tx::NoExtra)),
            Some(c) => self.calculate(&tx.clone().replace_extra(c.clone())),
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
