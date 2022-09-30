pub mod builder;

use crate::builder::settings::wallet::WalletType;
pub use builder::WalletTemplateBuilder;
use chain_addr::{AddressReadable, Discrimination};
use chain_impl_mockchain::value::Value;
use jormungandr_automation::jormungandr::NodeAlias;
use jormungandr_lib::interfaces::{DiscriminationDef, TokenIdentifier, ValueDef};
use serde::Deserialize;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};
use thor::{DiscriminationExtension, WalletAlias};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum WalletTemplate {
    /// Generated wallet when we want let hersir generate new wallet from scratch
    Generated {
        alias: WalletAlias,
        #[serde(with = "ValueDef")]
        value: Value,
        #[serde(default = "default_wallet_type")]
        wallet_type: WalletType,
        delegate: Option<NodeAlias>,
        #[serde(with = "DiscriminationDef")]
        #[serde(default = "default_discrimination")]
        discrimination: Discrimination,
        #[serde(default = "HashMap::new")]
        tokens: HashMap<TokenIdentifier, u64>,
    },
    /// Wallet which was given in configuration by address, thus hersir does not control it, which
    /// implies that some operations like delegation in block0 are not available
    External {
        address: String,
        #[serde(with = "ValueDef")]
        value: Value,
        #[serde(default = "HashMap::new")]
        tokens: HashMap<TokenIdentifier, u64>,
    },
}

pub fn default_wallet_type() -> WalletType {
    WalletType::Account
}

pub fn default_discrimination() -> Discrimination {
    Discrimination::Test
}

impl WalletTemplate {
    pub(crate) fn is_generated(&self) -> bool {
        matches!(self, Self::Generated { .. })
    }

    pub(crate) fn has_alias(&self, other_alias: &WalletAlias) -> bool {
        if let Some(alias) = &self.alias() {
            alias == other_alias
        } else {
            false
        }
    }
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for WalletTemplate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            WalletTemplate::Generated { alias, .. } => alias.hash(state),
            WalletTemplate::External { address, .. } => address.hash(state),
        }
    }
}

impl WalletTemplate {
    pub fn new_account<S: Into<WalletAlias>>(
        alias: S,
        value: Value,
        discrimination: Discrimination,
        tokens: HashMap<TokenIdentifier, u64>,
    ) -> Self {
        Self::Generated {
            alias: alias.into(),
            value,
            discrimination,
            tokens,
            wallet_type: WalletType::Account,
            delegate: None,
        }
    }
    pub fn new_utxo<S: Into<WalletAlias>>(
        alias: S,
        value: Value,
        discrimination: Discrimination,
        tokens: HashMap<TokenIdentifier, u64>,
    ) -> Self {
        Self::Generated {
            alias: alias.into(),
            value,
            discrimination,
            tokens,
            wallet_type: WalletType::UTxO,
            delegate: None,
        }
    }

    pub fn new_external<S: Into<String>>(
        address: S,
        value: Value,
        tokens: HashMap<TokenIdentifier, u64>,
    ) -> Self {
        Self::External {
            value,
            tokens,
            address: address.into(),
        }
    }

    pub fn id(&self) -> String {
        if let Some(alias) = self.alias() {
            alias
        } else if let Some(address) = self.address() {
            address
        } else {
            unreachable!()
        }
    }

    pub fn alias(&self) -> Option<WalletAlias> {
        match self {
            Self::External { .. } => None,
            Self::Generated { alias, .. } => Some(alias.clone()),
        }
    }

    pub fn address(&self) -> Option<String> {
        match self {
            Self::External { address, .. } => Some(address.clone()),
            Self::Generated { .. } => None,
        }
    }

    pub fn discrimination(&self) -> Discrimination {
        match self {
            Self::External { address, .. } => Discrimination::from_prefix(
                &AddressReadable::from_string_anyprefix(address)
                    .unwrap()
                    .get_prefix(),
            ),
            Self::Generated { discrimination, .. } => *discrimination,
        }
    }

    pub fn wallet_type(&self) -> Option<WalletType> {
        match self {
            Self::External { .. } => None,
            Self::Generated { wallet_type, .. } => Some(wallet_type.clone()),
        }
    }

    pub fn value(&self) -> &Value {
        match self {
            Self::External { value, .. } => value,
            Self::Generated { value, .. } => value,
        }
    }

    pub fn delegate(&self) -> &Option<NodeAlias> {
        match self {
            Self::External { .. } => &None,
            Self::Generated { delegate, .. } => delegate,
        }
    }

    pub fn delegate_mut(&mut self) -> &mut Option<NodeAlias> {
        match self {
            Self::External { .. } => unimplemented!(),
            Self::Generated { delegate, .. } => delegate,
        }
    }

    pub fn tokens(&self) -> &HashMap<TokenIdentifier, u64> {
        match self {
            Self::External { tokens, .. } => tokens,
            Self::Generated { tokens, .. } => tokens,
        }
    }
}
