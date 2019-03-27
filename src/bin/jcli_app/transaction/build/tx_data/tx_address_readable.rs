use chain_addr::{Address, AddressReadable};
use jcli_app::utils::serde_with_string;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

#[derive(Debug)]
pub struct TxAddressReadable(AddressReadable);

impl TxAddressReadable {
    pub fn to_address(&self) -> Address {
        self.0.to_address()
    }
}

impl<'de> Deserialize<'de> for TxAddressReadable {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        serde_with_string::deserialize(deserializer)
    }
}

impl Serialize for TxAddressReadable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_with_string::serialize(&self.0, serializer)
    }
}

impl FromStr for TxAddressReadable {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let addr = input
            .parse()
            .map_err(|e| format!("failed to parse address: {}", e))?;
        Ok(TxAddressReadable(addr))
    }
}
