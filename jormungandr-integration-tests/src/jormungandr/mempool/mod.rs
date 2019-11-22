use crate::common::{
    configuration::genesis_model::Fund,
    jcli_wrapper::{self, JCLITransactionWrapper},
    jormungandr::{ConfigurationBuilder, Starter},
    process_utils, startup,
};

use jormungandr_lib::interfaces::Mempool;
use std::time::Duration;

#[test]
pub fn test_log_ttl() {
    let sender = startup::create_new_account_address();
    let reciever = startup::create_new_account_address();

    let log_ttl_timeout = 15;
    let garbage_collection_interval = 2;
    let timeout_grace_period = garbage_collection_interval * 2;

    let config = ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            value: 1000000.into(),
            address: sender.address.clone(),
        }])
        .with_mempool(Mempool {
            fragment_ttl: Duration::from_secs(10).into(),
            log_ttl: Duration::from_secs(log_ttl_timeout).into(),
            garbage_collection_interval: Duration::from_secs(garbage_collection_interval).into(),
        })
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let transaction = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
        .assert_add_account(&sender.address, &100.into())
        .assert_add_output(&reciever.address, &100.into())
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();
    jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr.rest_address());

    process_utils::sleep(log_ttl_timeout + timeout_grace_period);
    assert!(jcli_wrapper::assert_get_rest_message_log(&jormungandr.rest_address()).is_empty());
}
