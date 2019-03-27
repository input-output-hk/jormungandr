use super::TxAddressReadable;
use chain_addr::Address;
use chain_impl_mockchain::txbuilder::TransactionBuilder;
use chain_impl_mockchain::value::Value;
use jcli_app::utils::SegmentParser;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize)]
pub struct TxOutput {
    address: TxAddressReadable,
    value: u64,
}

impl FromStr for TxOutput {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut parser = SegmentParser::new(input);
        let address: TxAddressReadable = parser.parse_next()?;
        let value = parser.parse_next()?;
        parser.finish()?;
        Ok(TxOutput { address, value })
    }
}

impl TxOutput {
    pub fn apply<E: Clone>(&self, builder: &mut TransactionBuilder<Address, E>) {
        builder.add_output(self.address.to_address(), Value(self.value));
    }
}
