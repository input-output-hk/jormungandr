use crate::interfaces::Value;
use chain_impl_mockchain::transaction::{Input, InputEnum, UtxoPointer};
use serde::{Deserialize, Serialize};

const INPUT_PTR_SIZE: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionInput {
    pub input: TransactionInputType,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionInputType {
    Account([u8; INPUT_PTR_SIZE]),
    Utxo([u8; INPUT_PTR_SIZE], u8),
}

impl From<TransactionInput> for Input {
    fn from(i: TransactionInput) -> Input {
        match i.input {
            TransactionInputType::Account(aid) => {
                Input::from_enum(InputEnum::AccountInput(aid.into(), i.value.into()))
            }
            TransactionInputType::Utxo(txptr, txid) => {
                Input::from_enum(InputEnum::UtxoInput(UtxoPointer {
                    output_index: txid,
                    transaction_id: txptr.into(),
                    value: i.value.into(),
                }))
            }
        }
    }
}

impl From<Input> for TransactionInput {
    fn from(i: Input) -> TransactionInput {
        match i.to_enum() {
            InputEnum::AccountInput(ai, value) => TransactionInput {
                input: TransactionInputType::Account(ai.into()),
                value: value.into(),
            },
            InputEnum::UtxoInput(utxoptr) => TransactionInput {
                input: TransactionInputType::Utxo(
                    utxoptr.transaction_id.into(),
                    utxoptr.output_index,
                ),
                value: utxoptr.value.into(),
            },
        }
    }
}
