use crate::common::{
    jcli_wrapper, jormungandr::ConfigurationBuilder, startup, transaction_utils::TransactionHash,
};

use chain_impl_mockchain::{rewards::TaxType, testing::TestGen};

use assert_fs::prelude::*;
use assert_fs::TempDir;
use chain_crypto::{Curve25519_2HashDH, SumEd25519_12};
use jormungandr_lib::{
    crypto::{hash::Hash, key::KeyPair},
    interfaces::{GenesisPraos, NodeSecret},
};
use jormungandr_testing_utils::testing::StepReporter;

#[test]
pub fn update_pool_fees_is_not_allowed() {
    let temp_dir = TempDir::new().unwrap();

    let mut stake_pool_owner = startup::create_new_account_address();

    let (jormungandr, stake_pools) = startup::start_stake_pool(
        &[stake_pool_owner.clone()],
        &[],
        &mut ConfigurationBuilder::new().with_storage(&temp_dir.child("storage")),
    )
    .unwrap();

    let stake_pool = stake_pools.iter().next().unwrap();

    let mut new_stake_pool = stake_pool.clone();
    let mut stake_pool_info = new_stake_pool.info_mut();
    stake_pool_info.rewards = TaxType::zero();

    // 6. send pool update certificate
    startup::sleep_till_next_epoch(1, &jormungandr.block0_configuration());

    let transaction = stake_pool_owner
        .issue_pool_update_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &stake_pool,
            &new_stake_pool,
        )
        .unwrap()
        .encode();

    jcli_wrapper::assert_transaction_rejected(
        &transaction,
        &jormungandr,
        "Pool update doesnt currently allow fees update",
    );
}

#[test]
pub fn update_pool_keys() {
    let mut test_steps = StepReporter::new();

    test_steps.step("1. Start jormungandr with single stake pool");

    let mut stake_pool_owner = startup::create_new_account_address();

    let (jormungandr, stake_pools) = startup::start_stake_pool(
        &[stake_pool_owner.clone()],
        &[],
        &mut ConfigurationBuilder::new().with_explorer(),
    )
    .unwrap();

    test_steps.step("2. Wait for next epoch");

    startup::sleep_till_next_epoch(1, &jormungandr.block0_configuration());

    test_steps.step("3. Send pool update certificate with new keys");

    let stake_pool = stake_pools.iter().next().unwrap();
    let new_kes_key: KeyPair<SumEd25519_12> = startup::create_new_key_pair();
    let new_vrf_key: KeyPair<Curve25519_2HashDH> = startup::create_new_key_pair();

    let mut new_stake_pool = stake_pool.clone();
    let mut stake_pool_info = new_stake_pool.info_mut();
    stake_pool_info.keys.kes_public_key = new_kes_key.identifier().into_public_key();
    stake_pool_info.keys.vrf_public_key = new_vrf_key.identifier().into_public_key();

    let transaction = stake_pool_owner
        .issue_pool_update_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &stake_pool,
            &new_stake_pool,
        )
        .unwrap()
        .encode();

    jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);

    test_steps.step("4. switch keys via demote/promote REST operations");

    jormungandr.rest().demote(1).expect("cannot demote leader");

    let secret = NodeSecret {
        bft: None,
        genesis: Some(GenesisPraos {
            node_id: Hash::from_hash(TestGen::hash()),
            sig_key: new_kes_key.signing_key(),
            vrf_key: new_vrf_key.signing_key(),
        }),
    };

    jormungandr
        .rest()
        .promote(secret)
        .expect("cannot promote leader");

    let block_count_created_with_old_keys = jormungandr.logger.get_created_blocks_counter();

    test_steps.step("5. Wait till next epoch");

    startup::sleep_till_next_epoch(1, &jormungandr.block0_configuration());

    test_steps.step("6. verify stake pool still create blocks");

    let block_count_created_with_new_keys = jormungandr.logger.get_created_blocks_counter();

    assert!(block_count_created_with_new_keys > block_count_created_with_old_keys);
}
