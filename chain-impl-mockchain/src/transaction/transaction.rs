use super::transfer::*;
use crate::key::Hash;
use crate::value::Value;
use chain_addr::Address;
use chain_core::mempack::{read_vec, ReadBuf, ReadError, Readable};
use chain_core::property;

// FIXME: should this be a wrapper type?
pub type TransactionId = Hash;

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

impl<Extra: Readable> Transaction<Address, Extra> {
    fn read_body<'a>(
        buf: &mut ReadBuf<'a>,
        num_inputs: usize,
        num_outputs: usize,
    ) -> Result<Self, ReadError> {
        let inputs = read_vec(buf, num_inputs)?;
        let outputs = read_vec(buf, num_outputs)?;
        let extra = Extra::read(buf)?;

        Ok(Transaction {
            inputs,
            outputs,
            extra,
        })
    }

    pub fn read_with_header<'a>(reader: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let num_inputs = reader.get_u8()? as usize;
        let num_outputs = reader.get_u8()? as usize;

        if !(num_inputs < 255) {
            return Err(ReadError::SizeTooBig(num_inputs, 255));
        }
        if !(num_outputs < 255) {
            return Err(ReadError::SizeTooBig(num_outputs, 255));
        }
        Self::read_body(reader, num_inputs, num_outputs)
    }
}

impl<Extra: property::Serialize> Transaction<Address, Extra> {
    fn serialize_body<W: std::io::Write>(&self, writer: &mut W) -> Result<(), Extra::Error> {
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
        self.extra.serialize(&mut codec)?;
        Ok(())
    }

    pub fn serialize_with_header<W: std::io::Write>(&self, writer: W) -> Result<(), Extra::Error> {
        use chain_core::packer::*;

        assert!(self.inputs.len() < 255);
        assert!(self.outputs.len() < 255);

        let mut codec = Codec::from(writer);

        // store the number of inputs and outputs
        codec.put_u8(self.inputs.len() as u8)?;
        codec.put_u8(self.outputs.len() as u8)?;

        self.serialize_body(&mut codec.into_inner())
    }

    pub fn hash(&self) -> TransactionId {
        let mut bytes = Vec::new();
        self.serialize_body(&mut bytes).unwrap();
        TransactionId::hash_bytes(&bytes)
    }
}

impl<Extra: property::Deserialize> Transaction<Address, Extra> {
    fn deserialize_body<R: std::io::BufRead>(
        reader: R,
        num_inputs: usize,
        num_outputs: usize,
    ) -> Result<Self, Extra::Error> {
        use chain_core::packer::*;
        use chain_core::property::Deserialize as _;
        let mut codec = Codec::from(reader);

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

        let extra = Extra::deserialize(&mut codec)?;

        Ok(Transaction {
            inputs,
            outputs,
            extra,
        })
    }

    pub fn deserialize_with_header<R: std::io::BufRead>(reader: R) -> Result<Self, Extra::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::from(reader);

        let num_inputs = codec.get_u8()? as usize;
        let num_outputs = codec.get_u8()? as usize;

        if num_inputs < 255 && num_outputs < 255 {
            Self::deserialize_body(codec.into_inner(), num_inputs, num_outputs)
        } else {
            // should return a nice error ...
            panic!("deserialization with 256 inputs/outputs")
        }
    }
}

impl<Extra: property::Serialize> property::Serialize for Transaction<Address, Extra> {
    type Error = Extra::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Extra::Error> {
        self.serialize_with_header(writer)
    }
}

impl<Extra: property::Deserialize> property::Deserialize for Transaction<Address, Extra> {
    type Error = Extra::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Extra::Error> {
        Self::deserialize_with_header(reader)
    }
}

impl<Extra: Readable> Readable for Transaction<Address, Extra> {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Self::read_with_header(buf)
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
}

impl property::TransactionId for TransactionId {}
