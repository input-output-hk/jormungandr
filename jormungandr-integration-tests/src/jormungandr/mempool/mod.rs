use crate::common::{
    configuration::{genesis_model::Fund, node_config_model::TrustedPeer},
    jcli_wrapper::{self, JCLITransactionWrapper},
    jormungandr::{ConfigurationBuilder, Starter},
    process_utils, startup,
};

use jormungandr_lib::interfaces::Mempool;
use std::time::Duration;

#[test]
pub fn test_log_ttl() {
    let mut sender = startup::create_new_account_address();
    let mut reciever = startup::create_new_account_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            value: 1000000.into(),
            address: sender.address.clone(),
        }])
        .with_trusted_peers(vec![TrustedPeer {
            address: "/ip4/13.230.137.72/tcp/3000".to_string(),
            id: "ed25519_pk1w6f2sclsauhfd6r9ydgvn0yvpvg4p3x3u2m2n7thknwghrfpdu5sgvrql9".to_string(),
        }])
        .with_mempool(Mempool {
            fragment_ttl: Duration::from_secs(10).into(),
            log_ttl: Duration::from_secs(10).into(),
            garbage_collection_interval: Duration::from_secs(2).into(),
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
    assert!(jcli_wrapper::assert_get_rest_message_log(&jormungandr.rest_address()).len() > 0);
    process_utils::sleep(10);
    assert!(jcli_wrapper::assert_get_rest_message_log(&jormungandr.rest_address()).is_empty());
}
