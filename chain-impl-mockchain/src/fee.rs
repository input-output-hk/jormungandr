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

pub trait FeeAlgorithm {
    fn calculate_for<Extra>(&self, tx: &tx::Transaction<Address, Extra>) -> Option<Value>;
}

impl FeeAlgorithm for LinearFee {
    fn calculate_for<Extra>(&self, tx: &tx::Transaction<Address, Extra>) -> Option<Value> {
        let msz = (tx.inputs.len() as u64).checked_add(tx.outputs.len() as u64)?;
        // FIXME for now we don't consider extra as payload, however in the near future
        // we need a trait related to the Extra that will give the fee valuation of the certificate
        let fee = self
            .coefficient
            .checked_mul(msz)?
            .checked_add(self.constant)?;
        Some(Value(fee))
    }
}
