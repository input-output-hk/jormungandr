use crate::legacy::OldAddress;
use crate::value::*;
use chain_core::mempack::{ReadBuf, ReadError, Readable};

/// Information how tokens are spent.
/// A value of tokens is sent to the address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Output<Address> {
    pub address: Address,
    pub value: Value,
}

impl<Address: Readable> Output<Address> {
    pub fn from_address(address: Address, value: Value) -> Self {
        Output { address, value }
    }
}

impl<Address: Readable> Readable for Output<Address> {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let address = Address::read(buf)?;
        let value = Value::read(buf)?;
        Ok(Output { address, value })
    }
}

impl std::fmt::Display for Output<chain_addr::Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}.{}", self.address.base32(), self.value)
    }
}

impl std::fmt::Display for Output<OldAddress> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}.{}", self.address, self.value)
    }
}
