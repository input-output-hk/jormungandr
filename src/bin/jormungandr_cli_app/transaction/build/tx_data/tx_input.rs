use chain_addr::Address;
use chain_impl_mockchain::transaction::{Input, TransactionId, UtxoPointer};
use chain_impl_mockchain::txbuilder::TransactionBuilder;
use chain_impl_mockchain::value::Value;
use jormungandr_cli_app::utils::{serde_with_string, SegmentParser};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize)]
pub struct TxInput {
    #[serde(with = "serde_with_string")]
    tx_id: TransactionId,
    tx_idx: u8,
    value: u64,
}

impl FromStr for TxInput {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut parser = SegmentParser::new(input);
        let tx_id: TransactionId = parser.parse_next()?;
        let tx_idx: u8 = parser.parse_next()?;
        let value: u64 = parser.parse_next()?;
        parser.finish()?;
        Ok(TxInput {
            tx_id,
            tx_idx,
            value,
        })
    }
}

impl TxInput {
    pub fn apply<E: Clone>(&self, builder: &mut TransactionBuilder<Address, E>) {
        let utxo_pointer = UtxoPointer::new(self.tx_id, self.tx_idx, Value(self.value));
        let input = Input::from_utxo(utxo_pointer);
        builder.add_input(&input);
    }
}
