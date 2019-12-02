use crate::testing::{
    verifiers::LedgerStateVerifier, scenario::{wallet,prepare_scenario}
};

#[test]
pub fn management_threshold() {
        let (mut ledger, controller) = prepare_scenario()
            .with_initials(vec![
                wallet("Alice").with(1_000).owns("stake_pool"),
                wallet("Bob").with(1_000).owns("stake_pool"),
                wallet("Clarice").with(1_000).owns("stake_pool"),
                wallet("David").with(1_000).owns("stake_pool"),
            ])
            .build()
            .unwrap();
        let alice = controller.wallet("Alice").unwrap();
        let bob = controller.wallet("Bob").unwrap();
        let stake_pool = controller.stake_pool("stake_pool").unwrap();

        // by default we need owners/2 votes to update pool
        assert!(controller.retire(&[&alice], &stake_pool, &mut ledger).is_err());

        LedgerStateVerifier::new(ledger.clone().into())
            .info("after owner delegation")
            .stake_pools()
                .is_not_retired(&stake_pool);

        assert!(controller.retire(&[&alice,&bob], &stake_pool, &mut ledger).is_ok());

        LedgerStateVerifier::new(ledger.clone().into())
            .info("after owner delegation")
            .stake_pools()
                .is_retired(&stake_pool);
    }