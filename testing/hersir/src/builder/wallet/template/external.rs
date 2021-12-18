use chain_impl_mockchain::value::Value;
use jormungandr_lib::interfaces::ValueDef;
use jormungandr_testing_utils::wallet::WalletAlias;
use serde::Deserialize;

/// Struct can be used to differentiate wallet template
/// which only adress is known and controller cannot control it
#[derive(Clone, Debug, Deserialize)]
pub struct ExternalWalletTemplate {
    alias: WalletAlias,
    address: String,
    #[serde(with = "ValueDef")]
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

    pub fn alias(&self) -> WalletAlias {
        self.alias.clone()
    }
}
