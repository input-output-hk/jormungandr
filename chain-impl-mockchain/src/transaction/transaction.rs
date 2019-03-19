use super::transfer::*;
use crate::{key::Hash, value::Value};
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

impl property::Serialize for Value {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        codec.put_u64(self.0)
    }
}

impl property::Serialize for Transaction<Address> {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::from(writer);

        assert!(self.inputs.len() < 255);
        assert!(self.outputs.len() < 255);

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

impl Transaction<Address> {
    pub fn hash(&self) -> TransactionId {
        use chain_core::packer::*;
        use chain_core::property::Serialize;

        let writer = Vec::new();
        let mut codec = Codec::from(writer);
        let bytes = {
            for input in self.inputs.iter() {
                input.serialize(&mut codec).unwrap();
            }
            for output in self.outputs.iter() {
                output.address.serialize(&mut codec).unwrap();
                output.value.serialize(&mut codec).unwrap();
            }
            codec.into_inner()
        };
        TransactionId::hash_bytes(&bytes)
    }
}

impl property::TransactionId for TransactionId {}
