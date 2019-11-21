use crate::{
    testing::{
        data::{Wallet,StakePool,AddressDataValue},
        ledger::{TestLedger,ConfigBuilder, LedgerBuilder},
        builders::{
            create_initial_stake_pool_registration,
            create_initial_stake_pool_delegation,
            StakePoolBuilder
        }
    },
    fragment::Fragment,
};

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
    config: Option<ConfigBuilder>,
    initials: Option<Vec<WalletTemplateBuilder>>
}


pub fn prepare_scenario() -> ScenarioBuilder {
    ScenarioBuilder{
        config: None,
        initials: None
    }
}

impl ScenarioBuilder {

    pub fn with_config(&mut self, config: ConfigBuilder) -> &mut Self {
        self.config = Some(config);
        self
    }

    pub fn with_initials(&mut self, initials: Vec<&mut WalletTemplateBuilder>) -> &mut Self {
        self.initials = Some(initials.iter().map(|x| (**x).clone()).collect());
        self
    }

    pub fn build(&self) -> Result<(TestLedger,Controller),ScenarioBuilderError> {
       
        if self.initials.is_none() {
           return Err(ScenarioBuilderError::UndefinedInitials)
        }

        let initials: Result<Vec<WalletTemplate>,ScenarioBuilderError> = self.initials.clone().unwrap().iter().cloned().map(|x| x.build()).collect();
        let config = self.config.clone().ok_or(ScenarioBuilderError::UndefinedConfig)?;
        let initials: Vec<WalletTemplate> = initials?;
        let wallets: Vec<Wallet> = initials.iter().cloned().map(|x| self.build_wallet(x)).collect();
        let stake_pools_wallet_map = StakePoolTemplateBuilder::new(&initials);
        let stake_pool_templates: Vec<StakePoolTemplate> = stake_pools_wallet_map.build_stake_pool_templates(wallets.clone())?;

        let stake_pools = self.build_stake_pools(stake_pool_templates);

        let mut messages = self.build_stake_pools_fragments(&initials,&stake_pools,&wallets);
        messages.extend(self.build_delegation_fragments(&initials,&stake_pools,&wallets));

        let faucets: Vec<AddressDataValue> = wallets.iter().cloned().map(|x| x.as_account()).collect();
        let test_ledger = LedgerBuilder::from_config(config)
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

    fn build_stake_pools_fragments(&self,initials: &Vec<WalletTemplate>, stake_pools: &Vec<StakePool>, wallets: &Vec<Wallet> ) -> Vec<Fragment> {
        initials.iter().cloned().filter(|x| x.owns_stake_pool().is_some())
            .map(|wallet_template|
                {
                    let stake_pool_alias = wallet_template.owns_stake_pool().unwrap();
                    let stake_pool = stake_pools.iter().find(|sp| sp.alias() == stake_pool_alias).unwrap();
                    let wallet_allias = wallet_template.alias();
                    let owners: Vec<Wallet> = wallets.iter().cloned().filter(|w| w.alias() == wallet_allias).collect();
                    create_initial_stake_pool_registration(&stake_pool,&owners)
                })
            .collect()
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
        StakePoolBuilder::new()
            .with_owners(template.owners())
            .with_alias(&template.alias())
            .build()
    }
}

pub fn wallet(alias: &str) -> WalletTemplateBuilder {
    WalletTemplateBuilder::new(alias)
}

