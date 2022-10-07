use crate::{signed_delegation_cert, signed_stake_pool_cert, StakePool, Wallet};
use jormungandr_automation::jormungandr::Block0ConfigurationBuilder;
use jormungandr_lib::interfaces::{Initial, InitialUTxO, Value};

pub trait Block0ConfigurationBuilderExtension {
    fn with_wallet(self, wallet: &Wallet, value: Value) -> Self;
    fn with_wallets_having_some_values(self, wallets: Vec<&Wallet>) -> Self;
    fn with_stake_pool(self, stake_pool: &StakePool) -> Self;
    fn with_delegation_to_stake_pool(self, stake_pool: &StakePool, wallets: Vec<&Wallet>) -> Self;
    fn with_stake_pool_and_delegation(self, stake_pool: &StakePool, wallets: Vec<&Wallet>) -> Self;
}

impl Block0ConfigurationBuilderExtension for Block0ConfigurationBuilder {
    fn with_wallet(self, wallet: &Wallet, value: Value) -> Self {
        self.with_funds(vec![Initial::Fund(vec![InitialUTxO {
            address: wallet.address(),
            value,
        }])])
    }

    fn with_wallets_having_some_values(mut self, wallets: Vec<&Wallet>) -> Self {
        for wallet in wallets {
            self = self.with_wallet(wallet, 1_000_000.into());
        }
        self
    }

    fn with_stake_pool(self, stake_pool: &StakePool) -> Self {
        self.with_certs(vec![Initial::Cert(
            signed_stake_pool_cert(
                chain_impl_mockchain::block::BlockDate {
                    epoch: 1,
                    slot_id: 0,
                },
                stake_pool,
            )
            .into(),
        )])
    }

    fn with_delegation_to_stake_pool(self, stake_pool: &StakePool, wallets: Vec<&Wallet>) -> Self {
        self.with_certs(
            wallets
                .iter()
                .map(|x| {
                    Initial::Cert(
                        signed_delegation_cert(
                            x,
                            chain_impl_mockchain::block::BlockDate {
                                epoch: 1,
                                slot_id: 0,
                            },
                            stake_pool.inner().id(),
                        )
                        .into(),
                    )
                })
                .collect(),
        )
    }

    fn with_stake_pool_and_delegation(
        self,
        stake_pool: &StakePool,
        delegators: Vec<&Wallet>,
    ) -> Self {
        self.with_stake_pool(stake_pool)
            .with_delegation_to_stake_pool(stake_pool, delegators)
    }
}
