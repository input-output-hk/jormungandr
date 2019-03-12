use super::utxo::{TransactionId, UtxoPointer};
use crate::account;
use crate::value::*;
use chain_crypto::PublicKey;

/// Generalized input which have a specific input value, and
/// either contains an account reference or a TransactionId+index
///
/// This uniquely refer to a specific source of value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Input {
    index_or_account: u8,
    value: Value,
    input_ptr: [u8; 32],
}

pub enum InputEnum {
    AccountInput(account::Identifier, Value),
    UtxoInput(UtxoPointer),
}

impl Input {
    pub fn to_enum(&self) -> InputEnum {
        if self.index_or_account == 0xff {
            let pk =
                PublicKey::from_bytes(&self.input_ptr).expect("internal error in publickey type");
            InputEnum::AccountInput(pk.into(), self.value)
        } else {
            InputEnum::UtxoInput(UtxoPointer::new(
                TransactionId::from_bytes(self.input_ptr.clone()),
                self.index_or_account,
                self.value,
            ))
        }
    }

    pub fn from_enum(ie: InputEnum) -> Input {
        match ie {
            InputEnum::AccountInput(id, value) => {
                let pk: PublicKey<account::AccountAlg> = id.into();
                let mut input_ptr = [0u8; 32];
                input_ptr.clone_from_slice(pk.as_ref());
                Input {
                    index_or_account: 0xff,
                    value: value,
                    input_ptr: input_ptr,
                }
            }

            InputEnum::UtxoInput(utxo_pointer) => {
                let mut input_ptr = [0u8; 32];
                input_ptr.clone_from_slice(utxo_pointer.transaction_id.as_ref());
                Input {
                    index_or_account: utxo_pointer.output_index,
                    value: utxo_pointer.value,
                    input_ptr: input_ptr,
                }
            }
        }
    }
}

/// Information how tokens are spent.
/// A value of tokens is sent to the address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Output<Address>(pub Address, pub Value);
