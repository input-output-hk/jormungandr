use crate::common::{
    jcli_wrapper::{self, jcli_transaction_wrapper::JCLITransactionWrapper},
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
};

use jormungandr_lib::interfaces::InitialUTxO;

use assert_fs::prelude::*;
use assert_fs::TempDir;

#[test]
pub fn two_nodes_communication() {
    let temp_dir = TempDir::new().unwrap();

    let sender = startup::create_new_utxo_address();
    let reciever = startup::create_new_utxo_address();

    let leader_dir = temp_dir.child("leader");
    leader_dir.create_dir_all().unwrap();
    let leader_config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&leader_dir);

    let leader_jormungandr = Starter::new()
        .config(leader_config.clone())
        .start()
        .unwrap();

    let trusted_node_dir = temp_dir.child("trusted_node");
    trusted_node_dir.create_dir_all().unwrap();
    let trusted_node_config = ConfigurationBuilder::new()
        .with_trusted_peers(vec![leader_jormungandr.to_trusted_peer()])
        .with_block_hash(leader_config.genesis_block_hash())
        .build(&trusted_node_dir);

    let trusted_jormungandr = Starter::new()
        .config(trusted_node_config.clone())
        .passive()
        .start()
        .unwrap();

    let utxo = leader_config.block0_utxo_for_address(&sender);
    let transaction_message = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        *utxo.associated_fund(),
        &sender,
        *utxo.associated_fund(),
        &reciever,
        trusted_node_config.genesis_block_hash(),
    );

    jcli_wrapper::assert_post_transaction(&transaction_message, &trusted_jormungandr.rest_uri());
}
