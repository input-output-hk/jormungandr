use crate::testing::network_builder::WalletAlias;
use chain_impl_mockchain::value::Value;

#[derive(Clone, Debug)]
pub struct LegacyWalletTemplate {
    alias: WalletAlias,
    address: String,
    value: Value,
    mnemonics: String,
}

impl LegacyWalletTemplate {
    #[inline]
    pub fn new<S: Into<WalletAlias>>(
        alias: S,
        value: Value,
        address: String,
        mnemonics: String,
    ) -> Self {
        Self {
            alias: alias.into(),
            value,
            address,
            mnemonics,
        }
    }

    pub fn alias(&self) -> &WalletAlias {
        &self.alias
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn address(&self) -> String {
        self.address.clone()
    }

    pub fn mnemonics(&self) -> String {
        self.mnemonics.clone()
    }
}
