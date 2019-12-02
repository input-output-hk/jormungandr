use crate::{
    fee::LinearFee,
    stake::Stake,
    testing::{ledger::ConfigBuilder, verifiers::LedgerStateVerifier, scenario::{wallet,prepare_scenario}},
    value::Value,
   
};
use chain_addr::Discrimination;

#[test]
pub fn owner_delegation() {
        let (mut ledger, controller) = prepare_scenario()
            .with_config(
                ConfigBuilder::new(0)
                    .with_discrimination(Discrimination::Test)
                    .with_fee(LinearFee::new(1, 1, 1))
            )
            .with_initials(vec![
                wallet("Alice").with(1_000),
                wallet("Bob").with(1_000).owns("stake_pool"),
            ])
            .build()
            .unwrap();
        let mut alice = controller.wallet("Alice").unwrap();
        let stake_pool = controller.stake_pool("stake_pool").unwrap();

        controller.owner_delegates(&alice, &stake_pool, &mut ledger).unwrap();
        alice.confirm_transaction();

        LedgerStateVerifier::new(ledger.clone().into())
            .info("after owner delegation")
            .distribution()
                .unassigned_is(Stake::from_value(Value(1000)))
                .and()
                .dangling_is(Stake::from_value(Value::zero()))
                .and()
                .pools_total_stake_is(Stake::from_value(Value(997)));

        controller.removes_delegation(&alice, &mut ledger).unwrap();
        alice.confirm_transaction();

        LedgerStateVerifier::new(ledger.into())
            .info("after owner delegation removal")
            .distribution()
                .unassigned_is(Stake::from_value(Value(1994)))
                .and()
                .dangling_is(Stake::from_value(Value::zero()))
                .and()
                .pools_total_stake_is(Stake::from_value(Value::zero()));
    }