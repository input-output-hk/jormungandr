use super::transfer::*;
use crate::key::Hash;
use crate::value::Value;
use chain_addr::Address;
use chain_core::property;

// FIXME: should this be a wrapper type?
pub type TransactionId = Hash;

/// Transaction, transaction maps old unspent tokens into the
/// set of the new addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction<OutAddress> {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output<OutAddress>>,
}

impl Transaction<Address> {
    fn serialize_body<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        use chain_core::packer::*;
        use chain_core::property::Serialize;

        let mut codec = Codec::from(writer);
        for input in self.inputs.iter() {
            input.serialize(&mut codec)?;
        }
        for output in self.outputs.iter() {
            output.address.serialize(&mut codec)?;
            output.value.serialize(&mut codec)?;
        }
        Ok(())
    }

    pub fn serialize_with_header<W: std::io::Write>(
        &self,
        writer: W,
    ) -> Result<(), std::io::Error> {
        use chain_core::packer::*;

        assert!(self.inputs.len() < 255);
        assert!(self.outputs.len() < 255);

        let mut codec = Codec::from(writer);

        // store the number of inputs and outputs
        codec.put_u8(self.inputs.len() as u8)?;
        codec.put_u8(self.outputs.len() as u8)?;

        self.serialize_body(&mut codec.into_inner())
    }

    fn deserialize_body<R: std::io::BufRead>(
        reader: R,
        num_inputs: usize,
        num_outputs: usize,
    ) -> Result<Self, std::io::Error> {
        use chain_core::packer::*;
        use chain_core::property::Deserialize as _;
        let mut codec = Codec::from(reader);

        let mut transaction = Transaction {
            inputs: Vec::with_capacity(num_inputs),
            outputs: Vec::with_capacity(num_outputs),
        };
        for _ in 0..num_inputs {
            let input = Input::deserialize(&mut codec)?;
            transaction.inputs.push(input);
        }

        for _ in 0..num_outputs {
            let address = Address::deserialize(&mut codec)?;
            let value = Value::deserialize(&mut codec)?;
            transaction.outputs.push(Output { address, value });
        }

        Ok(transaction)
    }

    pub fn deserialize_with_header<R: std::io::BufRead>(reader: R) -> Result<Self, std::io::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::from(reader);

        let num_inputs = codec.get_u8()? as usize;
        let num_outputs = codec.get_u8()? as usize;

        Self::deserialize_body(codec.into_inner(), num_inputs, num_outputs)
    }

    pub fn hash(&self) -> TransactionId {
        let mut bytes = Vec::new();
        self.serialize_body(&mut bytes).unwrap();
        TransactionId::hash_bytes(&bytes)
    }
}

impl property::Serialize for Transaction<Address> {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.serialize_with_header(writer)
    }
}

impl property::Deserialize for Transaction<Address> {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        Self::deserialize_with_header(reader)
    }
}

impl property::TransactionId for TransactionId {}
