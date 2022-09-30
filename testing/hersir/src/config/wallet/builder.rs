use crate::config::WalletTemplate;
use chain_addr::Discrimination;
use chain_impl_mockchain::value::Value;
use jormungandr_automation::jormungandr::NodeAlias;
use jormungandr_lib::interfaces::TokenIdentifier;
use std::collections::HashMap;
use thor::WalletAlias;

pub struct WalletTemplateBuilder {
    alias: Option<WalletAlias>,
    address: Option<String>,
    value: u64,
    node_alias: Option<NodeAlias>,
    discrimination: Discrimination,
    tokens: HashMap<TokenIdentifier, u64>,
}

impl Default for WalletTemplateBuilder {
    fn default() -> Self {
        Self {
            alias: None,
            address: None,
            value: 0u64,
            node_alias: None,
            discrimination: Discrimination::Test,
            tokens: HashMap::new(),
        }
    }
}

impl WalletTemplateBuilder {
    pub fn new<S: Into<WalletAlias>>(alias: S) -> Self {
        Self::default().with_alias(alias)
    }

    pub fn with_alias<S: Into<WalletAlias>>(mut self, alias: S) -> Self {
        self.alias = Some(alias.into());
        self
    }

    pub fn with_address<S: Into<String>>(mut self, address: S) -> Self {
        self.address = Some(address.into());
        self
    }

    pub fn with(mut self, value: u64) -> Self {
        self.value = value;
        self
    }

    pub fn with_token(mut self, id: TokenIdentifier, amount: u64) -> Self {
        self.tokens.insert(id, amount);
        self
    }

    pub fn discrimination(mut self, discrimination: Discrimination) -> Self {
        self.discrimination = discrimination;
        self
    }

    pub fn delegated_to(mut self, delegated_to: &str) -> Self {
        self.node_alias = Some(delegated_to.to_string());
        self
    }

    pub fn build(self) -> WalletTemplate {
        if let Some(alias) = self.alias {
            let mut wallet = WalletTemplate::new_account(
                alias,
                Value(self.value),
                self.discrimination,
                self.tokens,
            );

            *wallet.delegate_mut() = self.node_alias;
            wallet
        } else if let Some(address) = self.address {
            WalletTemplate::new_external(address, Value(self.value), self.tokens)
        } else {
            panic!("no alias nor address defined in wallet template builder. This didn't know which type to create");
        }
    }
}
