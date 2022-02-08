use crate::builder::WalletTemplate;
use chain_addr::Discrimination;
use chain_impl_mockchain::value::Value;
use jormungandr_automation::jormungandr::NodeAlias;
use jormungandr_lib::interfaces::TokenIdentifier;
use std::collections::HashMap;
use thor::WalletAlias;

pub struct WalletTemplateBuilder {
    alias: WalletAlias,
    value: u64,
    node_alias: Option<NodeAlias>,
    discrimination: Discrimination,
    tokens: HashMap<TokenIdentifier, u64>,
}

impl WalletTemplateBuilder {
    pub fn new(alias: &str) -> Self {
        Self {
            alias: alias.to_string(),
            value: 0u64,
            node_alias: None,
            discrimination: Discrimination::Test,
            tokens: HashMap::new(),
        }
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
        let mut wallet = WalletTemplate::new_account(
            self.alias.clone(),
            Value(self.value),
            self.discrimination,
            self.tokens,
        );
        *wallet.delegate_mut() = self.node_alias;
        wallet
    }
}
