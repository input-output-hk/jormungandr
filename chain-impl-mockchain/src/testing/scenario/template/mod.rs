mod builders;

pub use builders::*;

use crate::{
    value::Value,
    certificate::PoolPermissions
};
use chain_crypto::{PublicKey,Ed25519};



#[derive(Clone,Debug,Hash)]
pub struct WalletTemplate {
    pub alias: String,
    pub stake_pool_delegate_alias: Option<String>,
    pub stake_pool_owner_alias: Option<String>,
    pub initial_value: Value
}

impl PartialEq for WalletTemplate {
    fn eq(&self, other: &WalletTemplate) -> bool {
        self.alias == other.alias
    }
}

impl Eq for WalletTemplate {}

impl WalletTemplate {
    pub fn new(alias: &str, initial_value: Value) -> Self {
        WalletTemplate {
            alias: alias.to_owned(),
            stake_pool_delegate_alias: None,
            stake_pool_owner_alias: None,
            initial_value: initial_value
        }
    }

    pub fn delegates_stake_pool(&self) -> Option<String> {
        self.stake_pool_delegate_alias.clone()
    }

    pub fn owns_stake_pool(&self) -> Option<String> {
        self.stake_pool_owner_alias.clone()
    }

    pub fn alias(&self) -> String {
        self.alias.clone()
    }
}

#[derive(Clone,Debug)]
pub struct StakePoolTemplate {
    pub alias: String,
    pub owners: Vec<PublicKey<Ed25519>>,
}

impl StakePoolTemplate {
    pub fn alias(&self) -> String {
        self.alias.clone()
    }

    pub fn owners(&self) -> Vec<PublicKey<Ed25519>> {
        self.owners.clone()
    }
}

#[derive(Clone,Debug)]
pub struct StakePoolDef {
    pub name: String,
    pub permissions_threshold: Option<u8>,
}

impl StakePoolDef {
    pub fn pool_permission(&self) -> Option<PoolPermissions> {
        match self.permissions_threshold {
            Some(permissions_threshold) => Some(PoolPermissions::new(permissions_threshold)),
            None => None
        }
    }
}

