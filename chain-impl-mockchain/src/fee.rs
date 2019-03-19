use crate::transaction as tx;
use crate::value::Value;
use chain_addr::Address;

/// Linear fee using the basic affine formula
/// `COEFFICIENT * (COUNT(tx inputs) + COUNT(tx outputs)) + CONSTANT`.
#[derive(PartialEq, Eq, PartialOrd, Debug, Clone, Copy)]
pub struct LinearFee {
    pub constant: u64,
    pub coefficient: u64,
}

impl LinearFee {
    pub fn new(constant: u64, coefficient: u64) -> Self {
        LinearFee {
            constant,
            coefficient,
        }
    }
}

pub trait FeeAlgorithm {
    fn calculate_for(&self, tx: &tx::Transaction<Address>) -> Option<Value>;
}

impl FeeAlgorithm for LinearFee {
    fn calculate_for(&self, tx: &tx::Transaction<Address>) -> Option<Value> {
        let msz = (tx.inputs.len() as u64).checked_add(tx.outputs.len() as u64)?;
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?;
        Some(Value(fee))
    }
}
