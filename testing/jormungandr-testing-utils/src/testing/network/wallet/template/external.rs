use crate::testing::network::WalletAlias;
use crate::testing::serde::ValueSerde;
use chain_impl_mockchain::value::Value;
use serde::Deserialize;

/// Struct can be used to differentiate wallet template
/// which only adress is known and controller cannot control it
#[derive(Clone, Debug, Deserialize)]
pub struct ExternalWalletTemplate {
    alias: WalletAlias,
    address: String,
    #[serde(with = "ValueSerde")]
    value: Value,
}

impl ExternalWalletTemplate {
    #[inline]
    pub fn new<S: Into<WalletAlias>>(alias: S, value: Value, address: String) -> Self {
        Self {
            alias: alias.into(),
            value,
            address,
        }
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }
}
