use crate::{
    testing::{
        data::{Wallet,StakePool,AddressDataValue},
        ledger::{TestLedger,ConfigBuilder, LedgerBuilder},
        builders::{
            create_initial_stake_pool_registration,
            create_initial_stake_pool_delegation,
            StakePoolBuilder
        },
        scenario::template::StakePoolDef
    },
    fee::LinearFee,
    fragment::Fragment,
};
use chain_addr::Discrimination;

use super::{
    template::{StakePoolTemplateBuilder,WalletTemplateBuilder,WalletTemplate,StakePoolTemplate},
    Controller
};

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub ScenarioBuilderError
        UndefinedConfig = " no config defined",
        UndefinedInitials =  "no initials defined",
        NoOwnersForStakePool{ alias: String} = "stake pool '{alias}' must have at least one owner",
        UndefinedValueForWallet { alias: String }= "with(...) method must be used for '{alias}' wallet in scenario builder. "
}


pub struct ScenarioBuilder {
    config: ConfigBuilder,
    initials: Option<Vec<WalletTemplateBuilder>>,
    stake_pools_info_def: Vec<StakePoolDef>
}


pub fn prepare_scenario() -> ScenarioBuilder {

    let default_config_builder = ConfigBuilder::new(0)
        .with_discrimination(Discrimination::Test)
        .with_fee(LinearFee::new(1, 1, 1));

    ScenarioBuilder{
        config: default_config_builder,
        initials: None,
        stake_pools_info_def: Vec::new()
    }
}

impl ScenarioBuilder {

    pub fn with_config(&mut self, config: ConfigBuilder) -> &mut Self {
        self.config = config;
        self
    }

    pub fn with_initials(&mut self, initials: Vec<&mut WalletTemplateBuilder>) -> &mut Self {
        self.initials = Some(initials.iter().map(|x| (**x).clone()).collect());
        self
    }

    pub fn with_stake_pools(&mut self, stake_pools_info_def: Vec<StakePoolDef>)-> &mut Self {
        self.stake_pools_info_def.extend(stake_pools_info_def.iter().cloned());
        self
    }

    pub fn build(&self) -> Result<(TestLedger,Controller),ScenarioBuilderError> {
       
        if self.initials.is_none() {
           return Err(ScenarioBuilderError::UndefinedInitials)
        }

        let initials: Result<Vec<WalletTemplate>,ScenarioBuilderError> = self.initials.clone().unwrap().iter().cloned().map(|x| x.build()).collect();
        let initials: Vec<WalletTemplate> = initials?;
        let wallets: Vec<Wallet> = initials.iter().cloned().map(|x| self.build_wallet(x)).collect();
        let stake_pools_wallet_map = StakePoolTemplateBuilder::new(&initials);
        let stake_pool_templates: Vec<StakePoolTemplate> = stake_pools_wallet_map.build_stake_pool_templates(wallets.clone())?;
        let stake_pools = self.build_stake_pools(stake_pool_templates);
        let mut messages = self.build_stake_pools_fragments(&stake_pools,&wallets);
        messages.extend(self.build_delegation_fragments(&initials,&stake_pools,&wallets));
        let faucets: Vec<AddressDataValue> = wallets.iter().cloned().map(|x| x.as_account()).collect();
        let test_ledger = LedgerBuilder::from_config(self.config.clone())
            .faucets(&faucets)
            .certs(&messages)
            .build()
            .expect("cannot build test ledger");
        let block0_hash = test_ledger.block0_hash.clone();

        Ok((test_ledger,
           Controller {
            block0_hash: block0_hash,
            declared_wallets: wallets,
            declared_stake_pools: stake_pools
        }))
    }

    fn build_stake_pools_fragments(&self, stake_pools: &Vec<StakePool>, wallets: &Vec<Wallet> ) -> Vec<Fragment> {
        stake_pools.iter().cloned().map(|stake_pool| {
            let owners_keys = stake_pool.info().owners;
            let owners: Vec<Wallet> = owners_keys.iter().cloned().map(|pk| wallets.iter().cloned().find(|x| x.public_key() == pk).expect("unknown key")).collect();
            create_initial_stake_pool_registration(&stake_pool,&owners)
        }).collect()
    }

    fn build_delegation_fragments(&self,initials: &Vec<WalletTemplate>, stake_pools: &Vec<StakePool>, wallets: &Vec<Wallet> ) -> Vec<Fragment> {
        initials.iter().cloned().filter(|x| x.delegates_stake_pool().is_some())
            .map(|wallet_template|
                {
                    let stake_pool_alias = wallet_template.delegates_stake_pool().unwrap();
                    let stake_pool = stake_pools.iter().find(|sp| sp.alias() == stake_pool_alias).unwrap();
                    let wallet_allias = wallet_template.alias();
                    let wallet = wallets.iter().find(|w| w.alias() == wallet_allias).unwrap();
                    create_initial_stake_pool_delegation(&stake_pool,&wallet)
                })
            .collect()
    }

    fn build_wallet(&self,template: WalletTemplate) -> Wallet {
        Wallet::new(&template.alias(),template.initial_value)
    }

    fn build_stake_pools(&self, stake_pool_templates: Vec<StakePoolTemplate>) -> Vec<StakePool> {
        stake_pool_templates.iter().cloned().map(|x| self.build_stake_pool(x)).collect()
    }

    fn build_stake_pool(&self, template: StakePoolTemplate) -> StakePool {
        let stake_pool_def_opt = self.stake_pools_info_def.iter().find(|x| x.name == template.alias);
        let mut builder = StakePoolBuilder::new();
        builder.with_owners(template.owners());
        builder.with_alias(&template.alias());
        
        if let Some(stake_pool_def) = stake_pool_def_opt {
            if let Some(pool_permission) = stake_pool_def.pool_permission() {
                builder.with_pool_permissions(pool_permission);
            }
            
        }
        builder.build()
    }
}

pub fn wallet(alias: &str) -> WalletTemplateBuilder {
    WalletTemplateBuilder::new(alias)
}

