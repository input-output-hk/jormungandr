use crate::common::{
    jcli_wrapper::{self, JCLITransactionWrapper},
    jormungandr::ConfigurationBuilder,
    process_utils::Wait,
    startup,
};

use std::time::Duration;

#[test]
pub fn explorer_test() {
    let faucet = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();

    let mut config = ConfigurationBuilder::new();

    config.with_explorer();

    let (jormungandr, _) = startup::start_stake_pool(&faucet, &mut config).unwrap();

    let transaction =
        JCLITransactionWrapper::new_transaction(&jormungandr.config.genesis_block_hash)
            .assert_add_account(&faucet.address, &1_000.into())
            .assert_add_output(&receiver.address, &1_000.into())
            .assert_finalize()
            .seal_with_witness_for_address(&faucet)
            .assert_to_message();

    let wait = Wait::new(Duration::from_secs(2), 10);

    let fragment_id = jcli_wrapper::assert_transaction_in_block_with_wait(
        &transaction,
        &jormungandr.rest_address(),
        &wait,
    );

    let explorer = jormungandr.explorer();
    let explorer_transaction = explorer
        .get_transaction(fragment_id)
        .expect("non existing transaction");
    assert_eq!(
        fragment_id, explorer_transaction.id,
        "incorrect fragment id"
    );
}
