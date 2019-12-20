use crate::common::{
    jcli_wrapper::{self, JCLITransactionWrapper},
    jormungandr::ConfigurationBuilder,
    startup,
};

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

    let fragment_id =
        jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr.rest_address());

    let explorer = jormungandr.explorer();

    println!("{:?}", explorer.get_transaction(fragment_id));
}
