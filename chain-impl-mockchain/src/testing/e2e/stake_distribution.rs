use crate::{
    fee::LinearFee,
    testing::{ledger::ConfigBuilder, verifiers::LedgerStateVerifier, scenario::{wallet,prepare_scenario}},
    value::Value
};
use chain_addr::Discrimination;

#[test]
pub fn stake_distribution_to_many_stake_pools() {
        let (mut ledger, controller) = prepare_scenario()
            .with_config(
                ConfigBuilder::new(0)
                    .with_discrimination(Discrimination::Test)
                    .with_fee(LinearFee::new(1, 1, 1))
            )
            .with_initials(vec![
                wallet("Alice").with(1_000).owns("alice_stake_pool"),
                wallet("Bob").with(1_000).owns("bob_stake_pool"),
                wallet("Clarice").with(1_000).owns("clarice_stake_pool"),
                wallet("David").with(1_003)
            ])
            .build()
            .unwrap();
        
        let alice_stake_pool = controller.stake_pool("alice_stake_pool").unwrap();
        let bob_stake_pool = controller.stake_pool("bob_stake_pool").unwrap();
        let clarice_stake_pool = controller.stake_pool("clarice_stake_pool").unwrap();

        let david = controller.wallet("David").unwrap();

        let delegation_ratio = vec![
            (&alice_stake_pool,2u8),
            (&bob_stake_pool,3u8),
            (&clarice_stake_pool,5u8)
        ];

        controller.delegates_to_many(&david,&delegation_ratio,&mut ledger).unwrap();

        let expected_distribution = vec![
            (alice_stake_pool.id(),Value(200)),
            (bob_stake_pool.id(),Value(300)),
            (clarice_stake_pool.id(),Value(500))
        ];

        LedgerStateVerifier::new(ledger.clone().into())
            .info("after delegation to many stake pools")
            .distribution()
                .pools_distribution_is(expected_distribution);
}