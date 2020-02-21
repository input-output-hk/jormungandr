use crate::common::{
    jcli_wrapper::{self, JCLITransactionWrapper},
    jormungandr::{ConfigurationBuilder, Starter},
    process_utils, startup,
};

use jormungandr_lib::interfaces::{InitialUTxO, Mempool};
use std::time::{Duration, SystemTime};

#[test]
pub fn garbage_collection_interval() {
    let mut sender = startup::create_new_account_address();
    let reciever = startup::create_new_account_address();

    let garbage_collection_interval = 1;
    let wait = garbage_collection_interval * 2 * 60; //twice garbage_collection_interval time

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            value: 1000000.into(),
            address: sender.address(),
        }])
        .with_mempool(Mempool {
            pool_max_entries: 10_000usize.into(),
            fragment_ttl: Duration::from_secs(10).into(),
            log_max_entries: 100_000usize.into(),
            garbage_collection_interval: Duration::from_secs(garbage_collection_interval).into(),
        })
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let now = SystemTime::now();

    while now.elapsed().unwrap().as_secs() < wait {
        let transaction = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
            .assert_add_account(&sender.address().to_string(), &100.into())
            .assert_add_output(&reciever.address().to_string(), &100.into())
            .assert_finalize()
            .seal_with_witness_for_address(&sender)
            .assert_to_message();
        jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);
        sender.confirm_transaction();
    }
    jormungandr.assert_no_errors_in_log();
}
