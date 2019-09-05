use crate::common::configuration::genesis_model::Fund;
use crate::common::jcli_wrapper;
use crate::common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use crate::common::startup;

#[test]
#[ignore]
pub fn two_nodes_communication() {
    let sender = startup::create_new_utxo_address();
    let reciever = startup::create_new_utxo_address();

    let mut leader_config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let _leader_jormungandr = startup::start_jormungandr_node_as_leader(&mut leader_config);

    let mut trusted_node_config = startup::ConfigurationBuilder::new()
        .with_trusted_peers(vec![leader_config.node_config.p2p.public_address.clone()])
        .with_block_hash(leader_config.genesis_block_hash.clone())
        .build();

    let _trusted_jormungandr = startup::start_jormungandr_node_as_slave(&mut trusted_node_config);
    let leader_jormungandr_rest_address = leader_config.get_node_address();
    let trusted_jormungandr_rest_address = trusted_node_config.get_node_address();

    let utxo = startup::get_utxo_for_address(&sender, &trusted_jormungandr_rest_address);
    let transaction_message = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        &utxo.associated_fund(),
        &sender,
        &utxo.associated_fund(),
        &reciever,
        &trusted_node_config.genesis_block_hash,
    );

    jcli_wrapper::assert_post_transaction(&transaction_message, &trusted_jormungandr_rest_address);
}
