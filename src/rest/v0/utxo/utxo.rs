use chain_addr::Address;
use chain_impl_mockchain::{
    key::Hash,
    transaction::{Output, UtxoPointer},
    utxo::Entry,
    value::Value,
};

pub struct TxId(Hash);

#[derive(Serialize)]
pub struct Utxo {
    in_txid: TxId,
    in_idx: u8,
    out_addr: Address,
    out_value: Value,
}

impl<'a> From<Entry<'a, Address>> for Utxo {
    fn from(utxo_entry: Entry<'a, Address>) -> Self {
        Self {
            in_txid: TxId(utxo_entry.transaction_id),
            in_idx: utxo_entry.output_index,
            out_addr: utxo_entry.output.address.clone(),
            out_value: utxo_entry.output.value,
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
