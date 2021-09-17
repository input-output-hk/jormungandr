use crate::testing::network::WalletAlias;
use chain_impl_mockchain::value::Value;

/// Struct can be used to differentiate wallet template
/// which only adress is known and controller cannot control it
#[derive(Clone, Debug)]
pub struct ExternalWalletTemplate {
    alias: WalletAlias,
    address: String,
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
