use assert_fs::TempDir;
use chain_impl_mockchain::{
    accounting::account::{DelegationRatio, DelegationType},
    block::BlockDate,
    transaction::AccountIdentifier,
};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{download_last_n_releases, get_jormungandr_bin, ConfigurationBuilder, Starter},
    testing::time,
};
use thor::{FragmentSender, StakePool, TransactionHash};

#[test]
// Re-enable when rate of breaking changes subsides and we can maintain
// backward compatible releases again.
#[ignore]
pub fn test_legacy_node_all_fragments() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let legacy_release = download_last_n_releases(1).get(0).cloned().unwrap();
    let jormungandr = get_jormungandr_bin(&legacy_release, &temp_dir);

    let mut first_stake_pool_owner = thor::Wallet::default();
    let mut second_stake_pool_owner = thor::Wallet::default();
    let mut full_delegator = thor::Wallet::default();
    let mut split_delegator = thor::Wallet::default();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            first_stake_pool_owner.to_initial_fund(1_000_000),
            second_stake_pool_owner.to_initial_fund(2_000_000),
            full_delegator.to_initial_fund(2_000_000),
            split_delegator.to_initial_fund(2_000_000),
        ])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .jormungandr_app(jormungandr)
        .legacy(legacy_release.version())
        .config(config)
        .start()
        .expect("cannot start legacy jormungandr");

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDate::first().next_epoch().into(),
        Default::default(),
    );

    let fragment_builder = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    );

    // 1. send simple transaction
    let mut fragment = fragment_builder
        .transaction(
            &first_stake_pool_owner,
            second_stake_pool_owner.address(),
            1_000.into(),
        )
        .expect("cannot create fragment from transaction between first and second pool owner");

    fragment_sender
        .send_fragment(&mut first_stake_pool_owner, fragment, &jormungandr)
        .expect("fragment send error for transaction between first and second pool owner");

    let first_stake_pool = StakePool::new(&first_stake_pool_owner);

    // 2a). send pool registration certificate
    fragment = fragment_builder.stake_pool_registration(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(&mut first_stake_pool_owner, fragment, &jormungandr)
        .expect("error while sending registration certificate for first stake pool owner");

    let second_stake_pool = StakePool::new(&second_stake_pool_owner);

    // 2b). send pool registration certificate
    fragment =
        fragment_builder.stake_pool_registration(&second_stake_pool_owner, &second_stake_pool);

    fragment_sender
        .send_fragment(&mut second_stake_pool_owner, fragment, &jormungandr)
        .expect("error while sending registration certificate for second stake pool owner");

    let stake_pools_from_rest = jormungandr
        .rest()
        .stake_pools()
        .expect("cannot retrieve stake pools id from rest");
    assert!(
        stake_pools_from_rest.contains(&first_stake_pool.id().to_string()),
        "newly created first stake pools is not visible in node"
    );
    assert!(
        stake_pools_from_rest.contains(&second_stake_pool.id().to_string()),
        "newly created second stake pools is not visible in node"
    );

    // 3. send owner delegation certificate
    fragment = fragment_builder.owner_delegation(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(&mut first_stake_pool_owner, fragment, &jormungandr)
        .expect("error while sending owner delegation cert");

    let stake_pool_owner_info = jcli.rest().v0().account_stats(
        first_stake_pool_owner.address().to_string(),
        jormungandr.rest_uri(),
    );
    let stake_pool_owner_delegation: DelegationType =
        stake_pool_owner_info.delegation().clone().into();
    assert_eq!(
        stake_pool_owner_delegation,
        DelegationType::Full(first_stake_pool.id())
    );

    // 4. send full delegation certificate
    fragment = fragment_builder.delegation(&full_delegator, &first_stake_pool);

    fragment_sender
        .send_fragment(&mut full_delegator, fragment, &jormungandr)
        .unwrap();

    let full_delegator_info = jcli
        .rest()
        .v0()
        .account_stats(full_delegator.address().to_string(), jormungandr.rest_uri());
    let full_delegator_delegation: DelegationType = full_delegator_info.delegation().clone().into();
    assert_eq!(
        full_delegator_delegation,
        DelegationType::Full(first_stake_pool.id())
    );

    // 5. send split delegation certificate
    fragment = fragment_builder.delegation_to_many(
        &split_delegator,
        vec![(&first_stake_pool, 1u8), (&second_stake_pool, 1u8)],
    );

    fragment_sender
        .send_fragment(&mut split_delegator, fragment, &jormungandr)
        .unwrap();

    let split_delegator = jcli.rest().v0().account_stats(
        split_delegator.address().to_string(),
        jormungandr.rest_uri(),
    );
    let delegation_ratio = DelegationRatio::new(
        2,
        vec![(first_stake_pool.id(), 1u8), (second_stake_pool.id(), 1u8)],
    )
    .unwrap();
    let split_delegator_delegation: DelegationType = split_delegator.delegation().clone().into();
    assert_eq!(
        split_delegator_delegation,
        DelegationType::Ratio(delegation_ratio)
    );

    let mut new_stake_pool = first_stake_pool.clone();
    let mut stake_pool_info = new_stake_pool.info_mut();

    stake_pool_info.reward_account = Some(AccountIdentifier::Single(
        second_stake_pool_owner
            .identifier()
            .into_public_key()
            .into(),
    ));

    // 6. send pool update certificate

    time::wait_for_epoch(2, jormungandr.rest());
    fragment = fragment_builder.stake_pool_update(
        vec![&first_stake_pool_owner],
        &first_stake_pool,
        &new_stake_pool,
    );

    jcli.fragment_sender(&jormungandr)
        .send(&fragment.encode())
        .assert_in_block();
    first_stake_pool_owner.confirm_transaction();

    // 7. send pool retire certificate
    fragment = fragment_builder.stake_pool_retire(vec![&first_stake_pool_owner], &first_stake_pool);

    fragment_sender
        .send_fragment(&mut first_stake_pool_owner, fragment, &jormungandr)
        .unwrap();

    let stake_pools_from_rest = jormungandr
        .rest()
        .stake_pools()
        .expect("cannot retrieve stake pools id from rest");
    assert!(
        !stake_pools_from_rest.contains(&first_stake_pool.id().to_string()),
        "newly created stake pools is not visible in node"
    );
}
