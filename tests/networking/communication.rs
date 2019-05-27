#![cfg(feature = "integration-test")]

use common::configuration::genesis_model::Fund;
use common::configuration::node_config_model::Peer;
use common::jcli_wrapper;
use common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use common::startup;

#[test]
pub fn two_nodes_communication() {
    let sender = startup::create_new_utxo_address();
    let reciever = startup::create_new_utxo_address();

    let mut leader_config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100,
        }])
        .build();

    let _leader_jormungandr = startup::start_jormungandr_node_as_leader(&mut leader_config);

    let mut trusted_node_config = startup::ConfigurationBuilder::new()
        .with_trusted_peers(vec![Peer {
            id: 1,
            address: leader_config.node_config.peer_2_peer.public_address.clone(),
        }])
        .with_block_hash(leader_config.genesis_block_hash.clone())
        .build();

    let _trusted_jormungandr = startup::start_jormungandr_node_as_slave(&mut trusted_node_config);
    let leader_jormungandr_rest_address = leader_config.get_node_address();
    let trusted_jormungandr_rest_address = trusted_node_config.get_node_address();

    let utxo = startup::get_utxo_for_address(&sender, &trusted_jormungandr_rest_address);
    let transaction_message = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        &utxo.out_value,
        &sender,
        &utxo.out_value,
        &reciever,
        &trusted_node_config.genesis_block_hash,
    )
    .assert_transaction_to_message();
    jcli_wrapper::assert_transaction_post_accepted(
        &transaction_message,
        &leader_jormungandr_rest_address,
    );

    println!("Leader");
    jcli_wrapper::assert_rest_stats(&leader_jormungandr_rest_address);

    println!("Trusted");
    jcli_wrapper::assert_rest_stats(&trusted_jormungandr_rest_address);
}
