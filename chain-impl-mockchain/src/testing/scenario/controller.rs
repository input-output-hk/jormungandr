use crate::{
    certificate::Certificate,
    date::BlockDate,
    key::Hash,
    ledger::Error as LedgerError,
    testing::{
        builders::{
            build_owner_stake_delegation, build_stake_delegation_cert,
            build_stake_pool_registration_cert, build_stake_pool_retirement_cert, TestTxBuilder,
            TestTxCertBuilder,
        },
        data::{StakePool, Wallet},
        ledger::TestLedger,
    },
    value::Value,
};

use super::scenario_builder::{prepare_scenario, wallet};
use chain_addr::Discrimination;

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub ControllerError
        UnknownWallet { alias: String } = "cannot find wallet with alias {alias}",
        UnknownStakePool { alias: String } = "cannot find stake pool with alias {alias}",
}

pub struct Controller {
    pub block0_hash: Hash,
    pub declared_wallets: Vec<Wallet>,
    pub declared_stake_pools: Vec<StakePool>,
}

impl Controller {
    pub fn wallet(&self, alias: &str) -> Result<Wallet, ControllerError> {
        self.declared_wallets
            .iter()
            .cloned()
            .find(|x| x.alias() == alias)
            .ok_or(ControllerError::UnknownWallet {
                alias: alias.to_owned(),
            })
    }

    /*
    fn empty_context() -> HeaderContentEvalContext {
        HeaderContentEvalContext {
            block_date: BlockDate::first(),
            chain_length: ChainLength(0),
            nonce: None,
        }
    }
    */

    pub fn stake_pool(&self, alias: &str) -> Result<StakePool, ControllerError> {
        self.declared_stake_pools
            .iter()
            .cloned()
            .find(|x| x.alias() == alias)
            .ok_or(ControllerError::UnknownStakePool {
                alias: alias.to_owned(),
            })
    }

    pub fn transfer_funds(
        &self,
        from: &Wallet,
        to: &Wallet,
        mut test_ledger: &mut TestLedger,
        funds: u64,
    ) -> Result<(), LedgerError> {
        let transaction = TestTxBuilder::new(&test_ledger.block0_hash)
            .move_funds(
                &mut test_ledger,
                &from.as_account(),
                &to.as_account(),
                &Value(funds),
            )
            .get_fragment();
        test_ledger.apply_transaction(transaction)
    }

    pub fn register(
        &self,
        funder: &Wallet,
        stake_pool: &StakePool,
        ledger: &mut TestLedger,
    ) -> Result<(), LedgerError> {
        let cert = build_stake_pool_registration_cert(&stake_pool.info());
        self.apply_transaction_with_cert(&[funder], cert, ledger)
    }

    pub fn delegates(
        &self,
        from: &Wallet,
        stake_pool: &StakePool,
        ledger: &mut TestLedger,
    ) -> Result<(), LedgerError> {
        let cert = build_stake_delegation_cert(&stake_pool.info(), &from.as_account_data());
        self.apply_transaction_with_cert(&[from], cert, ledger)
    }

    pub fn owner_delegates(
        &self,
        from: &Wallet,
        stake_pool: &StakePool,
        ledger: &mut TestLedger,
    ) -> Result<(), LedgerError> {
        let cert = build_owner_stake_delegation(stake_pool.id());
        self.apply_transaction_with_cert(&[from], cert, ledger)
    }

    pub fn retire(
        &self,
        owners: &[&Wallet],
        stake_pool: &StakePool,
        ledger: &mut TestLedger,
    ) -> Result<(), LedgerError> {
        let certificate = build_stake_pool_retirement_cert(stake_pool.id(), 0);
        self.apply_transaction_with_cert(&owners, certificate, ledger)
    }

    fn apply_transaction_with_cert(
        &self,
        wallets: &[&Wallet],
        certificate: Certificate,
        test_ledger: &mut TestLedger,
    ) -> Result<(), LedgerError> {
        let fragment = TestTxCertBuilder::new(&test_ledger).make_transaction(wallets, &certificate);
        test_ledger.apply_fragment(&fragment, BlockDate::first())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        fee::LinearFee,
        stake::Stake,
        testing::{ledger::ConfigBuilder, verifiers::LedgerStateVerifier},
        value::Value,
    };

    #[test]
    pub fn build_scenario_example() {
        let (mut ledger, controller) = prepare_scenario()
            .with_config(
                ConfigBuilder::new(0)
                    .with_discrimination(Discrimination::Test)
                    .with_fee(LinearFee::new(1, 1, 1))
            )
            .with_initials(vec![
                wallet("Alice").with(1_000).delegates_to("stake_pool"),
                wallet("Bob").with(1_000),
                wallet("Clarice").with(1_000).owns("stake_pool"),
            ])
            .build()
            .unwrap();
        let mut alice = controller.wallet("Alice").unwrap();
        let mut bob = controller.wallet("Bob").unwrap();
        let mut clarice = controller.wallet("Clarice").unwrap();
        let stake_pool = controller.stake_pool("stake_pool").unwrap();

        controller
            .transfer_funds(&alice, &bob, &mut ledger, 100)
            .unwrap();
        alice.confirm_transaction();
        controller
            .delegates(&bob, &stake_pool, &mut ledger)
            .unwrap();
        bob.confirm_transaction();
        controller
            .retire(&[&clarice], &stake_pool, &mut ledger)
            .unwrap();
        clarice.confirm_transaction();
        // unassigned = clarice - fee (becaue thus clarise is an onwer of the stake she did not delegates any stakes)
        // dangling = bob and alice funds (minus fees for transactions and certs)
        // total pool = 0, because stake pool was retired

        LedgerStateVerifier::new(ledger.into())
            .distribution()
            .unassigned_is(Stake::from_value(Value(997)))
            .and()
            .dangling_is(Stake::from_value(Value(1994)))
            .and()
            .pools_total_stake_is(Stake::zero());
    }
}
