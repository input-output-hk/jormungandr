use chain_addr::Address;
use chain_impl_mockchain::{
    key::Hash,
    transaction::{Output, UtxoPointer},
    value::Value,
};

pub struct TxId(Hash);

#[derive(Serialize)]
pub struct Utxo {
    in_txid: TxId,
    in_idx: u32,
    out_addr: Address,
    out_value: Value,
}

impl<'a> From<(&'a UtxoPointer, &'a Output)> for Utxo {
    fn from((pointer, output): (&'a UtxoPointer, &'a Output)) -> Self {
        Self {
            in_txid: TxId(pointer.transaction_id),
            in_idx: pointer.output_index,
            out_addr: output.0.clone(),
            out_value: output.1,
        }
    }
}

impl serde::ser::Serialize for TxId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        if serializer.is_human_readable() {
            let hex = cardano::util::hex::encode(self.0.as_ref());
            serializer.serialize_str(&hex)
        } else {
            serializer.serialize_bytes(self.0.as_ref())
        }
    }
}
