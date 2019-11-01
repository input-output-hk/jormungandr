use crate::common::{
    configuration::{genesis_model::Fund, node_config_model::TrustedPeer},
    jcli_wrapper::{self, jcli_transaction_wrapper::JCLITransactionWrapper},
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
};

#[test]
#[ignore]
pub fn two_nodes_communication() {
    let sender = startup::create_new_utxo_address();
    let reciever = startup::create_new_utxo_address();

    let leader_config = ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let _leader_jormungandr = Starter::new()
        .config(leader_config.clone())
        .start()
        .unwrap();

    let trusted_node_config = ConfigurationBuilder::new()
        .with_trusted_peers(vec![TrustedPeer {
            address: leader_config.node_config.p2p.public_address.clone(),
            id: leader_config.node_config.p2p.public_id.clone(),
        }])
        .with_block_hash(leader_config.genesis_block_hash.clone())
        .build();

    let trusted_jormungandr = Starter::new()
        .config(trusted_node_config.clone())
        .passive()
        .start()
        .unwrap();

    let utxo = startup::get_utxo_for_address(&sender, &trusted_jormungandr.rest_address());
    let transaction_message = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        &utxo.associated_fund(),
        &sender,
        &utxo.associated_fund(),
        &reciever,
        &trusted_node_config.genesis_block_hash,
    );

    jcli_wrapper::assert_post_transaction(
        &transaction_message,
        &trusted_jormungandr.rest_address(),
    );
}
