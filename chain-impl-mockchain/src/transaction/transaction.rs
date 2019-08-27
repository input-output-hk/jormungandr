use super::transfer::*;
use crate::value::{Value, ValueError};
use chain_addr::Address;
use chain_core::mempack::{read_vec, ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::{digest::DigestOf, Blake2b256};
use std::boxed::Box;

pub struct TransactionSignData(Box<[u8]>);

impl From<Vec<u8>> for TransactionSignData {
    fn from(v: Vec<u8>) -> TransactionSignData {
        TransactionSignData(v.into())
    }
}

impl AsRef<[u8]> for TransactionSignData {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

pub type TransactionSignDataHash = DigestOf<Blake2b256, TransactionSignData>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoExtra;

impl property::Serialize for NoExtra {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, _: W) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl property::Deserialize for NoExtra {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(_: R) -> Result<Self, Self::Error> {
        Ok(NoExtra)
    }
}
impl Readable for NoExtra {
    fn read<'a>(_: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(NoExtra)
    }
}

/// Transaction, transaction maps old unspent tokens into the
/// set of the new addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction<OutAddress, Extra> {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output<OutAddress>>,
    pub extra: Extra,
}

/// Amount of the balance in the transaction.
pub enum Balance {
    /// Balance is positive.
    Positive(Value),
    /// Balance is negative, such transaction can't be valid.
    Negative(Value),
    /// Balance is zero.
    Zero,
}

impl<Extra: property::Serialize> Transaction<Address, Extra> {
    pub fn hash(&self) -> TransactionSignDataHash {
        use chain_core::property::Serialize;
        let bytes = self.serialize_as_vec().unwrap(); // unwrap is safe when serializing to Vec
        DigestOf::digest(&TransactionSignData(bytes.into()))
    }
}

impl<Extra: property::Deserialize> Transaction<Address, Extra> {
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Extra::Error> {
        use chain_core::packer::*;
        use chain_core::property::Deserialize as _;
        let mut codec = Codec::new(reader);

        let extra = Extra::deserialize(&mut codec)?;

        let num_inputs = codec.get_u8()? as usize;
        let num_outputs = codec.get_u8()? as usize;

        let mut inputs = Vec::with_capacity(num_inputs);
        let mut outputs = Vec::with_capacity(num_outputs);
        for _ in 0..num_inputs {
            let input = Input::deserialize(&mut codec)?;
            inputs.push(input);
        }

        for _ in 0..num_outputs {
            let address = Address::deserialize(&mut codec)?;
            let value = Value::deserialize(&mut codec)?;
            outputs.push(Output { address, value });
        }

        Ok(Transaction {
            inputs,
            outputs,
            extra,
        })
    }
}

impl<Extra: property::Serialize> property::Serialize for Transaction<Address, Extra> {
    type Error = Extra::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Extra::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::new(writer);
        self.extra.serialize(&mut codec)?;

        // store the number of inputs and outputs
        codec.put_u8(self.inputs.len() as u8)?;
        codec.put_u8(self.outputs.len() as u8)?;

        for input in self.inputs.iter() {
            input.serialize(&mut codec)?;
        }
        for output in self.outputs.iter() {
            output.address.serialize(&mut codec)?;
            output.value.serialize(&mut codec)?;
        }
        Ok(())
    }
}

impl<Extra: property::Deserialize> property::Deserialize for Transaction<Address, Extra> {
    type Error = Extra::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Extra::Error> {
        Self::deserialize(reader)
    }
}

impl<Extra: Readable> Readable for Transaction<Address, Extra> {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let extra = Extra::read(buf)?;

        let num_inputs = buf.get_u8()? as usize;
        let num_outputs = buf.get_u8()? as usize;
        let inputs = read_vec(buf, num_inputs)?;
        let outputs = read_vec(buf, num_outputs)?;

        Ok(Transaction {
            inputs,
            outputs,
            extra,
        })
    }
}

impl<A, Extra> Transaction<A, Extra> {
    pub fn replace_extra<Extra2>(self, e2: Extra2) -> Transaction<A, Extra2> {
        Transaction {
            inputs: self.inputs,
            outputs: self.outputs,
            extra: e2,
        }
    }

    pub fn total_input(&self) -> Result<Value, ValueError> {
        Value::sum(self.inputs.iter().map(|input| input.value))
    }

    pub fn total_output(&self) -> Result<Value, ValueError> {
        Value::sum(self.outputs.iter().map(|output| output.value))
    }

    pub fn balance(&self, fee: Value) -> Result<Balance, ValueError> {
        let inputs = self.total_input()?;
        let outputs = self.total_output()?;
        let z = (outputs + fee)?;
        if inputs > z {
            Ok(Balance::Positive((inputs - z)?))
        } else if inputs < z {
            Ok(Balance::Negative((z - inputs)?))
        } else {
            Ok(Balance::Zero)
        }
    }
}
