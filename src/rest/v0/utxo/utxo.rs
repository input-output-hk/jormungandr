use chain_addr::Address;
use chain_impl_mockchain::{
    key::Hash,
    transaction::{Output, UtxoPointer},
    value::Value,
};

#[derive(Serialize)]
pub struct Utxo {
    in_txid: Hash,
    in_idx: u32,
    out_addr: Address,
    out_value: Value,
}

impl<'a> From<(&'a UtxoPointer, &'a Output)> for Utxo {
    fn from((pointer, output): (&'a UtxoPointer, &'a Output)) -> Self {
        Self {
            in_txid: pointer.transaction_id,
            in_idx: pointer.output_index,
            out_addr: output.0.clone(),
            out_value: output.1,
        }
    }
}
