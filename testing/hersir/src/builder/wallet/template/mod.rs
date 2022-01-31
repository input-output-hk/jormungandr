pub mod builder;
mod external;

use super::WalletType;
use chain_addr::Discrimination;
use chain_impl_mockchain::value::Value;
pub use external::ExternalWalletTemplate;
use jormungandr_automation::jormungandr::NodeAlias;
use jormungandr_lib::interfaces::{DiscriminationDef, TokenIdentifier, ValueDef};
use serde::Deserialize;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use thor::WalletAlias;
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WalletTemplate {
    alias: WalletAlias,
    #[serde(with = "ValueDef")]
    value: Value,
    wallet_type: WalletType,
    delegate: Option<NodeAlias>,
    #[serde(with = "DiscriminationDef")]
    discrimination: Discrimination,
    tokens: HashMap<TokenIdentifier, u64>,
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for WalletTemplate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.alias.hash(state)
    }
}

impl WalletTemplate {
    pub fn new_account<S: Into<WalletAlias>>(
        alias: S,
        value: Value,
        discrimination: Discrimination,
        tokens: HashMap<TokenIdentifier, u64>,
    ) -> Self {
        Self::new(alias, value, WalletType::Account, discrimination, tokens)
    }
    pub fn new_utxo<S: Into<WalletAlias>>(
        alias: S,
        value: Value,
        discrimination: Discrimination,
        tokens: HashMap<TokenIdentifier, u64>,
    ) -> Self {
        Self::new(alias, value, WalletType::UTxO, discrimination, tokens)
    }

    #[inline]
    fn new<S: Into<WalletAlias>>(
        alias: S,
        value: Value,
        wallet_type: WalletType,
        discrimination: Discrimination,
        tokens: HashMap<TokenIdentifier, u64>,
    ) -> Self {
        Self {
            alias: alias.into(),
            value,
            wallet_type,
            delegate: None,
            discrimination,
            tokens,
        }
    }

    pub fn alias(&self) -> &WalletAlias {
        &self.alias
    }

    pub fn discrimination(&self) -> Discrimination {
        self.discrimination
    }

    pub fn wallet_type(&self) -> &WalletType {
        &self.wallet_type
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn delegate(&self) -> &Option<NodeAlias> {
        &self.delegate
    }

    pub fn delegate_mut(&mut self) -> &mut Option<NodeAlias> {
        &mut self.delegate
    }

    pub fn tokens(&self) -> &HashMap<TokenIdentifier, u64> {
        &self.tokens
    }
}
