use crate::common::{
    jcli::JCli, jormungandr::ConfigurationBuilder, startup, transaction_utils::TransactionHash,
};
use chain_impl_mockchain::rewards::TaxType;
use jormungandr_testing_utils::testing::node::time;

use assert_fs::prelude::*;
use assert_fs::TempDir;

#[test]
pub fn update_pool_fees_is_not_allowed() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let mut stake_pool_owner = startup::create_new_account_address();

    let (jormungandr, stake_pools) = startup::start_stake_pool(
        &[stake_pool_owner.clone()],
        &[],
        &mut ConfigurationBuilder::new().with_storage(&temp_dir.child("storage")),
    )
    .unwrap();

    let stake_pool = stake_pools.get(0).unwrap();

    let mut new_stake_pool = stake_pool.clone();
    let mut stake_pool_info = new_stake_pool.info_mut();
    stake_pool_info.rewards = TaxType::zero();

    // 6. send pool update certificate
    time::wait_for_epoch(2, jormungandr.rest());

    let transaction = stake_pool_owner
        .issue_pool_update_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            chain_impl_mockchain::block::BlockDate {
                epoch: 3,
                slot_id: 0,
            },
            stake_pool,
            &new_stake_pool,
        )
        .unwrap()
        .encode();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction)
        .assert_rejected("Pool update doesnt currently allow fees update");
}
