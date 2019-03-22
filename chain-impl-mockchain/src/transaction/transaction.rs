use super::transfer::*;
use crate::key::Hash;
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
    pub fn serialize_body<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
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

impl property::TransactionId for TransactionId {}
