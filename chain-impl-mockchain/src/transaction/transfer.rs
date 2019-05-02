use super::transaction::TransactionId;
use super::utxo::UtxoPointer;
use crate::account;
use crate::value::*;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::PublicKey;

const INPUT_PTR_SIZE: usize = 32;

/// Generalized input which have a specific input value, and
/// either contains an account reference or a TransactionId+index
///
/// This uniquely refer to a specific source of value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Input {
    pub index_or_account: u8,
    pub value: Value,
    pub input_ptr: [u8; INPUT_PTR_SIZE],
}

pub enum InputType {
    Utxo,
    Account,
}

pub enum InputEnum {
    AccountInput(account::Identifier, Value),
    UtxoInput(UtxoPointer),
}

impl Input {
    pub fn get_type(&self) -> InputType {
        if self.index_or_account == 0xff {
            InputType::Account
        } else {
            InputType::Utxo
        }
    }

    pub fn from_utxo(utxo_pointer: UtxoPointer) -> Self {
        let mut input_ptr = [0u8; INPUT_PTR_SIZE];
        input_ptr.clone_from_slice(utxo_pointer.transaction_id.as_ref());
        Input {
            index_or_account: utxo_pointer.output_index,
            value: utxo_pointer.value,
            input_ptr: input_ptr,
        }
    }

    pub fn from_account(id: account::Identifier, value: Value) -> Self {
        let pk: PublicKey<account::AccountAlg> = id.into();
        let mut input_ptr = [0u8; INPUT_PTR_SIZE];
        input_ptr.clone_from_slice(pk.as_ref());
        Input {
            index_or_account: 0xff,
            value: value,
            input_ptr: input_ptr,
        }
    }

    pub fn to_enum(&self) -> InputEnum {
        match self.get_type() {
            InputType::Account => {
                let pk = PublicKey::from_binary(&self.input_ptr)
                    .expect("internal error in publickey type");
                InputEnum::AccountInput(pk.into(), self.value)
            }
            InputType::Utxo => InputEnum::UtxoInput(UtxoPointer::new(
                TransactionId::from_bytes(self.input_ptr.clone()),
                self.index_or_account,
                self.value,
            )),
        }
    }

    pub fn from_enum(ie: InputEnum) -> Input {
        match ie {
            InputEnum::AccountInput(id, value) => Self::from_account(id, value),
            InputEnum::UtxoInput(utxo_pointer) => Self::from_utxo(utxo_pointer),
        }
    }
}

impl property::Serialize for Input {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::new(writer);
        codec.put_u8(self.index_or_account)?;
        self.value.serialize(&mut codec)?;
        codec.into_inner().write_all(&self.input_ptr)?;
        Ok(())
    }
}

impl property::Deserialize for Input {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::new(reader);
        let index_or_account = codec.get_u8()?;
        let value = Value::deserialize(&mut codec)?;
        let mut input_ptr = [0; INPUT_PTR_SIZE];
        codec.into_inner().read_exact(&mut input_ptr)?;
        Ok(Input {
            index_or_account: index_or_account,
            value: value,
            input_ptr: input_ptr,
        })
    }
}

impl Readable for Input {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let index_or_account = buf.get_u8()?;
        let value = Value::read(buf)?;
        let input_ptr = <[u8; INPUT_PTR_SIZE]>::read(buf)?;
        Ok(Input {
            index_or_account: index_or_account,
            value: value,
            input_ptr: input_ptr,
        })
    }
}

/// Information how tokens are spent.
/// A value of tokens is sent to the address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Output<Address> {
    pub address: Address,
    pub value: Value,
}

impl<Address: Readable> Readable for Output<Address> {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let address = Address::read(buf)?;
        let value = Value::read(buf)?;
        Ok(Output { address, value })
    }
}
