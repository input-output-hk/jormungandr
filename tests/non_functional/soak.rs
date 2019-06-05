#![cfg(feature = "integration-test")]

use common::configuration::genesis_model::Fund;
use common::configuration::jormungandr_config::JormungandrConfig;
use common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use common::startup;

use common::jcli_wrapper;
use common::process_utils;

#[test]
#[cfg(feature = "soak-test")]
pub fn test_100_transaction_is_processed() {
    let mut sender = startup::create_new_utxo_address();
    let mut reciever = startup::create_new_utxo_address();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100,
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);

    for i in 0..100 {
        let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);

        let transaction_message =
            JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
                .assert_add_input_from_utxo(&utxo)
                .assert_add_output(&reciever.address.clone(), &utxo.out_value)
                .assert_finalize()
                .seal_with_witness_deafult(&sender.private_key.clone(), "utxo")
                .assert_transaction_to_message();
        jcli_wrapper::assert_transaction_post_accepted(
            &transaction_message,
            &jormungandr_rest_address,
        );

        std::mem::swap(&mut sender, &mut reciever);
        process_utils::sleep(1);
    }

    process_utils::sleep(1);
    let message_logs = jcli_wrapper::assert_rest_message_logs(&jormungandr_rest_address);
    message_logs.iter().for_each(|el| assert!(el.is_in_block()));
}
