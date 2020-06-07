use crate::common::{
    jcli_wrapper, jormungandr::ConfigurationBuilder, startup, transaction_utils::TransactionHash,
};
use jormungandr_testing_utils::stake_pool::StakePool;

use chain_impl_mockchain::accounting::account::{DelegationRatio, DelegationType};

use assert_fs::prelude::*;
use assert_fs::TempDir;

#[test]
pub fn test_all_fragments() {
    let temp_dir = TempDir::new().unwrap();

    let mut faucet = startup::create_new_account_address();
    let mut stake_pool_owner = startup::create_new_account_address();
    let mut full_delegator = startup::create_new_account_address();
    let mut split_delegator = startup::create_new_account_address();

    let stake_pool_owner_stake = 1_000;

    let (jormungandr, stake_pools) = startup::start_stake_pool(
        &[faucet.clone()],
        &[full_delegator.clone(), split_delegator.clone()],
        &mut ConfigurationBuilder::new().with_storage(&temp_dir.child("storage")),
    )
    .unwrap();

    let initial_stake_pool = stake_pools.iter().next().unwrap();

    // 1. send simple transaction
    let mut transaction = faucet
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            stake_pool_owner.address(),
            stake_pool_owner_stake.into(),
        )
        .unwrap()
        .encode();
    jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);

    let stake_pool = StakePool::new(&stake_pool_owner);

    // 2. send pool registration certificate
    transaction = stake_pool_owner
        .issue_pool_registration_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &stake_pool,
        )
        .unwrap()
        .encode();

    jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);
    stake_pool_owner.confirm_transaction();

    let stake_pools_from_rest = jormungandr
        .rest()
        .stake_pools()
        .expect("cannot retrieve stake pools id from rest");
    assert!(
        stake_pools_from_rest.contains(&stake_pool.id().to_string()),
        "newly created stake pools is not visible in node"
    );

    // 3. send owner delegation certificate
    transaction = stake_pool_owner
        .issue_owner_delegation_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &stake_pool,
        )
        .unwrap()
        .encode();

    jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);
    stake_pool_owner.confirm_transaction();

    let stake_pool_owner_info = jcli_wrapper::assert_rest_account_get_stats(
        &stake_pool_owner.address().to_string(),
        &jormungandr.rest_uri(),
    );
    let stake_pool_owner_delegation: DelegationType =
        stake_pool_owner_info.delegation().clone().into();
    assert_eq!(
        stake_pool_owner_delegation,
        DelegationType::Full(stake_pool.id())
    );

    // 4. send full delegation certificate
    transaction = full_delegator
        .issue_full_delegation_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &stake_pool,
        )
        .unwrap()
        .encode();

    jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);

    let full_delegator_info = jcli_wrapper::assert_rest_account_get_stats(
        &full_delegator.address().to_string(),
        &jormungandr.rest_uri(),
    );
    let full_delegator_delegation: DelegationType = full_delegator_info.delegation().clone().into();
    assert_eq!(
        full_delegator_delegation,
        DelegationType::Full(stake_pool.id())
    );

    // 5. send split delegation certificate
    transaction = split_delegator
        .issue_split_delegation_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            vec![(initial_stake_pool, 1u8), (&stake_pool, 1u8)],
        )
        .unwrap()
        .encode();

    jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);

    let split_delegator = jcli_wrapper::assert_rest_account_get_stats(
        &split_delegator.address().to_string(),
        &jormungandr.rest_uri(),
    );
    let delegation_ratio = DelegationRatio::new(
        2,
        vec![(initial_stake_pool.id(), 1u8), (stake_pool.id(), 1u8)],
    )
    .unwrap();
    let split_delegator_delegation: DelegationType = split_delegator.delegation().clone().into();
    assert_eq!(
        split_delegator_delegation,
        DelegationType::Ratio(delegation_ratio)
    );

    let mut new_stake_pool = stake_pool.clone();
    let mut stake_pool_info = new_stake_pool.info_mut();
    stake_pool_info.serial = 100u128;

    // 6. send pool update certificate
    startup::sleep_till_next_epoch(1, &jormungandr.block0_configuration());

    transaction = stake_pool_owner
        .issue_pool_update_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &stake_pool,
            &new_stake_pool,
        )
        .unwrap()
        .encode();

    jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);
    stake_pool_owner.confirm_transaction();

    // 7. send pool retire certificate
    transaction = stake_pool_owner
        .issue_pool_retire_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &stake_pool,
        )
        .unwrap()
        .encode();

    jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);

    let stake_pools_from_rest = jormungandr
        .rest()
        .stake_pools()
        .expect("cannot retrieve stake pools id from rest");
    assert!(
        !stake_pools_from_rest.contains(&stake_pool.id().to_string()),
        "newly created stake pools is not visible in node"
    );
}
