use crate::common::{
    jcli_wrapper, jormungandr::ConfigurationBuilder, process_utils, startup,
    transaction_utils::TransactionHash,
};
use jormungandr_lib::time::SystemTime;
#[test]
pub fn test_pool_update() {
    let mut faucet = startup::create_new_account_address();
    let (jormungandr, stake_pools) =
        startup::start_stake_pool(&[faucet.clone()], &[], &mut ConfigurationBuilder::new())
            .unwrap();

    process_utils::sleep(5);
    let created_block_count = jormungandr.logger.get_created_blocks_hashes().len();

    assert!(created_block_count > 0);

    let stake_pool = stake_pools.iter().cloned().next().unwrap();
    let transaction = faucet
        .issue_pool_retire_cert(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            &stake_pool,
        )
        .unwrap()
        .encode();

    jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);
    startup::sleep_till_next_epoch(1, &jormungandr.block0_configuration());

    let created_blocks_count_after_retire = jormungandr.logger.get_created_blocks_hashes().len();
    assert!(created_blocks_count_after_retire > created_block_count);

    let start_time = SystemTime::now();
    assert!(jormungandr
        .logger
        .get_created_blocks_hashes_after(start_time)
        .is_empty());
}
