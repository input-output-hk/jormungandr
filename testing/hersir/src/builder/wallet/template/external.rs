use std::collections::HashMap;

use chain_impl_mockchain::value::Value;
use jormungandr_lib::interfaces::{TokenIdentifier, ValueDef};
use serde::Deserialize;
use thor::WalletAlias;

/// Struct can be used to differentiate wallet template
/// which only adress is known and controller cannot control it
#[derive(Clone, Debug, Deserialize)]
pub struct ExternalWalletTemplate {
    alias: WalletAlias,
    address: String,
    #[serde(with = "ValueDef")]
    value: Value,
    tokens: HashMap<TokenIdentifier, u64>,
}

impl ExternalWalletTemplate {
    #[inline]
    pub fn new<S: Into<WalletAlias>>(
        alias: S,
        value: Value,
        address: String,
        tokens: HashMap<TokenIdentifier, u64>,
    ) -> Self {
        Self {
            alias: alias.into(),
            value,
            address,
            tokens,
        }
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn tokens(&self) -> &HashMap<TokenIdentifier, u64> {
        &self.tokens
    }

    pub fn alias(&self) -> WalletAlias {
        self.alias.clone()
    }
}
