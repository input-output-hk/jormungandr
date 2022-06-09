use crate::startup;
use assert_fs::{prelude::*, TempDir};
use chain_impl_mockchain::rewards::TaxType;
use jormungandr_automation::{jcli::JCli, jormungandr::ConfigurationBuilder, testing::time};
use thor::TransactionHash;

#[test]
pub fn update_pool_fees_is_not_allowed() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let stake_pool_owner = thor::Wallet::default();

    let (jormungandr, stake_pools) = startup::start_stake_pool(
        &[stake_pool_owner.clone()],
        &[],
        ConfigurationBuilder::new().with_storage(&temp_dir.child("storage")),
    )
    .unwrap();

    let stake_pool = stake_pools.get(0).unwrap();

    let mut new_stake_pool = stake_pool.clone();
    let mut stake_pool_info = new_stake_pool.info_mut();
    stake_pool_info.rewards = TaxType::zero();

    // 6. send pool update certificate
    time::wait_for_epoch(2, jormungandr.rest());

    let transaction = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        chain_impl_mockchain::block::BlockDate {
            epoch: 3,
            slot_id: 0,
        },
    )
    .stake_pool_update(vec![&stake_pool_owner], stake_pool, &new_stake_pool)
    .encode();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction)
        .assert_rejected("Pool update doesnt currently allow fees update");
}
