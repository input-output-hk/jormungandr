pub mod builder;
mod external;

use super::{WalletAlias, WalletType};
use crate::testing::network::NodeAlias;
use chain_addr::Discrimination;
use chain_impl_mockchain::value::Value;
pub use external::ExternalWalletTemplate;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct WalletTemplate {
    alias: WalletAlias,
    value: Value,
    wallet_type: WalletType,
    delegate: Option<NodeAlias>,
    discrimination: Discrimination,
}

impl WalletTemplate {
    pub fn new_account<S: Into<WalletAlias>>(
        alias: S,
        value: Value,
        discrimination: Discrimination,
    ) -> Self {
        Self::new(alias, value, WalletType::Account, discrimination)
    }
    pub fn new_utxo<S: Into<WalletAlias>>(
        alias: S,
        value: Value,
        discrimination: Discrimination,
    ) -> Self {
        Self::new(alias, value, WalletType::UTxO, discrimination)
    }

    #[inline]
    fn new<S: Into<WalletAlias>>(
        alias: S,
        value: Value,
        wallet_type: WalletType,
        discrimination: Discrimination,
    ) -> Self {
        Self {
            alias: alias.into(),
            value,
            wallet_type,
            delegate: None,
            discrimination,
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
}
