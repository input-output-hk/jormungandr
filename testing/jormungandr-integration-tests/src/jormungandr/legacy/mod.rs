use crate::common::{
    jcli_wrapper,
    jormungandr::ConfigurationBuilder,
    legacy::{self, download_last_n_releases, get_jormungandr_bin, Version},
    startup,
};
use jormungandr_testing_utils::{
    stake_pool::StakePool,
    testing::{FragmentNode, FragmentSender},
};

use chain_impl_mockchain::accounting::account::{DelegationRatio, DelegationType};

use jormungandr_lib::interfaces::InitialUTxO;
use std::str::FromStr;

#[ignore]
#[test]
pub fn test_legacy_node_all_fragments() {
    let legacy_release = download_last_n_releases(1).iter().cloned().next().unwrap();
    let jormungandr = get_jormungandr_bin(&legacy_release);
    let version = Version::from_str(&legacy_release.version()).unwrap();

    let mut first_stake_pool_owner = startup::create_new_account_address();
    let mut second_stake_pool_owner = startup::create_new_account_address();
    let mut full_delegator = startup::create_new_account_address();
    let mut split_delegator = startup::create_new_account_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            InitialUTxO {
                address: first_stake_pool_owner.address(),
                value: 200.into(),
            },
            InitialUTxO {
                address: second_stake_pool_owner.address(),
                value: 1_000_000.into(),
            },
        ])
        .build();

    let jormungandr = legacy::Starter::new(version, jormungandr)
        .config(config)
        .start()
        .unwrap();

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        Default::default(),
    );

    // 1. send simple transaction
    let mut fragment = first_stake_pool_owner
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            second_stake_pool_owner.address(),
            1_000.into(),
        )
        .unwrap();

    fragment_sender
        .send_fragment(fragment, &jormungandr)
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(30));

    let first_stake_pool = StakePool::new(&first_stake_pool_owner);

    // 2a). send pool registration certificate
    fragment = first_stake_pool_owner
        .issue_pool_registration_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &first_stake_pool,
        )
        .unwrap();

    fragment_sender
        .send_fragment(fragment, &jormungandr)
        .unwrap();
    first_stake_pool_owner.confirm_transaction();

    let second_stake_pool = StakePool::new(&second_stake_pool_owner);

    // 2b). send pool registration certificate
    fragment = first_stake_pool_owner
        .issue_pool_registration_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &second_stake_pool,
        )
        .unwrap();

    fragment_sender
        .send_fragment(fragment, &jormungandr)
        .unwrap();
    second_stake_pool_owner.confirm_transaction();

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
    fragment = first_stake_pool_owner
        .issue_owner_delegation_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &first_stake_pool,
        )
        .unwrap();

    fragment_sender
        .send_fragment(fragment, &jormungandr)
        .unwrap();
    first_stake_pool_owner.confirm_transaction();

    let stake_pool_owner_info = jcli_wrapper::assert_rest_account_get_stats(
        &first_stake_pool_owner.address().to_string(),
        &jormungandr.rest_address(),
    );
    let stake_pool_owner_delegation: DelegationType =
        stake_pool_owner_info.delegation().clone().into();
    assert_eq!(
        stake_pool_owner_delegation,
        DelegationType::Full(first_stake_pool.id())
    );

    // 4. send full delegation certificate
    fragment = full_delegator
        .issue_full_delegation_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &first_stake_pool,
        )
        .unwrap();

    fragment_sender
        .send_fragment(fragment, &jormungandr)
        .unwrap();

    let full_delegator_info = jcli_wrapper::assert_rest_account_get_stats(
        &full_delegator.address().to_string(),
        &jormungandr.rest_address(),
    );
    let full_delegator_delegation: DelegationType = full_delegator_info.delegation().clone().into();
    assert_eq!(
        full_delegator_delegation,
        DelegationType::Full(first_stake_pool.id())
    );

    // 5. send split delegation certificate
    fragment = split_delegator
        .issue_split_delegation_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            vec![(&first_stake_pool, 1u8), (&second_stake_pool, 1u8)],
        )
        .unwrap();

    fragment_sender
        .send_fragment(fragment, &jormungandr)
        .unwrap();

    let split_delegator = jcli_wrapper::assert_rest_account_get_stats(
        &split_delegator.address().to_string(),
        &jormungandr.rest_address(),
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

    /*
        let mut new_stake_pool = stake_pool.clone();
        let mut stake_pool_info = new_stake_pool.info_mut();
        stake_pool_info.rewards = TaxType::zero();

        // 6. send pool update certificate

        startup::sleep_till_next_epoch(1, &jormungandr.config);

        transaction = stake_pool_owner.issue_pool_update_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &stake_pool,
            &new_stake_pool
        ).unwrap()
        .encode();

        jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);
        stake_pool_owner.confirm_transaction();
    */
    // 7. send pool retire certificate
    fragment = first_stake_pool_owner
        .issue_pool_retire_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &first_stake_pool,
        )
        .unwrap();

    fragment_sender
        .send_fragment(fragment, &jormungandr)
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
