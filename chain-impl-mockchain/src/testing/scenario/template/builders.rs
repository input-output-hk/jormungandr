use crate::{
    value::Value,
    testing::scenario::scenario_builder::ScenarioBuilderError,
    testing::data::Wallet,

};

use super::{
    WalletTemplate,StakePoolTemplate
};

use std::collections::{HashMap,HashSet};

#[derive(Clone,Debug)]
pub struct WalletTemplateBuilder{
    alias: String,
    delagate_alias: Option<String>,
    ownership_alias: Option<String>,
    initial_value: Option<Value>
}

impl WalletTemplateBuilder{

    pub fn new(alias: &str) -> Self {
        WalletTemplateBuilder{
            alias: alias.to_owned(),
            delagate_alias: None,
            ownership_alias: None,
            initial_value: None,
        }
    }

    pub fn with(&mut self, value: u64) -> &mut Self {
        self.initial_value = Some(Value(value));
        self
    }

    pub fn owns(&mut self, ownership_alias: &str) -> &mut Self {
        self.ownership_alias = Some(ownership_alias.to_owned());
        self
    }

   pub fn delegates_to(&mut self, delegates_to_alias: &str) -> &mut Self {
        self.delagate_alias = Some(delegates_to_alias.to_owned());
        self
    }

    pub fn build(&self) -> Result<WalletTemplate,ScenarioBuilderError> {
        let value = self.initial_value.ok_or(ScenarioBuilderError::UndefinedValueForWallet{
            alias: self.alias.clone()
        })?;

        Ok(WalletTemplate {
            alias: self.alias.clone(),
            stake_pool_delegate_alias: self.delagate_alias.clone(),
            stake_pool_owner_alias: self.ownership_alias.clone(),
            initial_value: value.clone()
        })
    }
}


pub struct StakePoolTemplateBuilder{
    ownership_map:  HashMap<String,HashSet<WalletTemplate>>,
    delegation_map:  HashMap<String,HashSet<WalletTemplate>>,
}

impl StakePoolTemplateBuilder{

    pub fn new(initials: &Vec<WalletTemplate>) -> Self {
        StakePoolTemplateBuilder {
            ownership_map: Self::build_ownersip_map(initials),
            delegation_map: Self::build_delegation_map(initials)
        }
    }

    pub fn build_stake_pool_templates(&self, wallets: Vec<Wallet>) -> Result<Vec<StakePoolTemplate>,ScenarioBuilderError>{
       self.defined_stake_pools_aliases().iter().map(|stake_pool_alias| {
            let owners = self.ownership_map.get(stake_pool_alias).ok_or(ScenarioBuilderError::NoOwnersForStakePool{alias: stake_pool_alias.to_string()})?;

            let owners_public_keys = wallets.iter()
                    .filter(|w| owners.iter().any(|u| u.alias() == w.alias()))
                    .map(|w| w.public_key())
                    .collect();

            Ok(StakePoolTemplate{
                alias: stake_pool_alias.to_string(),
                owners: owners_public_keys
            })
        }).collect()
    }

    pub fn defined_stake_pools_aliases(&self) -> HashSet<String> {
        self.ownership_map.clone().into_iter().chain(self.delegation_map.clone()).map(|(k,_)| k).collect()
    }

    fn build_ownersip_map(initials: &Vec<WalletTemplate>) -> HashMap<String,HashSet<WalletTemplate>> {
        let mut output: HashMap<String,HashSet<WalletTemplate>> = HashMap::new();
        for wallet_template in initials.iter().filter(|w| w.owns_stake_pool().is_some()) {
            let delegate_alias = wallet_template.owns_stake_pool().unwrap();
            match output.contains_key(&delegate_alias) {
                true => {output.get_mut(&delegate_alias).unwrap().insert(wallet_template.clone());}
                false => {
                    let mut delegation_aliases = HashSet::new();
                    delegation_aliases.insert(wallet_template.clone());
                    output.insert(delegate_alias,delegation_aliases);
                }
            }
        }
        output
    }

    fn build_delegation_map( initials: &Vec<WalletTemplate>) ->  HashMap<String,HashSet<WalletTemplate>> {
        let mut output:  HashMap<String,HashSet<WalletTemplate>> = HashMap::new();
        for wallet_template in initials.iter().filter(|w| w.delegates_stake_pool().is_some()) {
            let stake_pool_alias = wallet_template.delegates_stake_pool().unwrap();
            match output.contains_key(&stake_pool_alias) {
                true => {output.get_mut(&stake_pool_alias).unwrap().insert(wallet_template.clone());}
                false => {
                    let mut ownership_aliases = HashSet::new();
                    ownership_aliases.insert(wallet_template.clone());
                    output.insert(stake_pool_alias.to_string(),ownership_aliases);
                }
            }
        }
        output
    }
}