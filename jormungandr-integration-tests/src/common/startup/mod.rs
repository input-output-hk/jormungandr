mod configuration_builder;

use crate::common::configuration::genesis_model::GenesisYaml;

use crate::common::data::{
    address::{Account, AddressDataProvider, Delegation, Utxo},
    keys::KeyPair,
};

use crate::common::file_utils;
use crate::common::jormungandr::starter;
use jormungandr_lib::interfaces::UTxOInfo;
use std::path::PathBuf;

use crate::common::jcli_wrapper;
use crate::common::jcli_wrapper::Discrimination;

pub use self::configuration_builder::ConfigurationBuilder;

pub use self::starter::{
    assert_start_jormungandr_node_as_passive_fail, start_jormungandr_node,
    start_jormungandr_node_as_leader, start_jormungandr_node_as_passive,
    start_jormungandr_node_as_slave,
};

pub fn get_genesis_block_hash(genesis_yaml: &GenesisYaml) -> String {
    let path_to_output_block = build_genesis_block(&genesis_yaml);

    jcli_wrapper::assert_genesis_hash(&path_to_output_block)
}

pub fn build_genesis_block(genesis_yaml: &GenesisYaml) -> PathBuf {
    let input_yaml_file_path = GenesisYaml::serialize(&genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);

    path_to_output_block
}

pub fn create_new_utxo_address() -> Utxo {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_single(&public_key, Discrimination::Test);
    let utxo = Utxo {
        private_key,
        public_key,
        address,
    };
    utxo
}

pub fn create_new_account_address() -> Account {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_account(&public_key, Discrimination::Test);
    let account = Account {
        private_key,
        public_key,
        address,
    };
    account
}

pub fn create_new_delegation_address() -> Delegation {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_single(&public_key, Discrimination::Test);

    let private_delegation_key = jcli_wrapper::assert_key_generate_default();
    let public_delegation_key = jcli_wrapper::assert_key_to_public_default(&private_delegation_key);
    let delegation_address =
        jcli_wrapper::assert_address_single(&public_delegation_key, Discrimination::Test);

    let utxo_with_delegation = Delegation {
        private_key,
        public_key,
        address,
        delegation_address,
    };
    println!(
        "New utxo with delegation generated: {:?}",
        &utxo_with_delegation
    );
    utxo_with_delegation
}

pub fn create_new_delegation_address_for(delegation_address: &str) -> Delegation {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_single(&public_key, Discrimination::Test);

    let utxo_with_delegation = Delegation {
        private_key: private_key,
        public_key: public_key,
        address: address,
        delegation_address: delegation_address.to_string(),
    };
    println!(
        "New utxo with delegation generated: {:?}",
        &utxo_with_delegation
    );
    utxo_with_delegation
}

pub fn create_new_key_pair(key_type: &str) -> KeyPair {
    let private_key = jcli_wrapper::assert_key_generate(&key_type);
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    KeyPair {
        private_key,
        public_key,
    }
}

pub fn get_utxo_for_address<T: AddressDataProvider>(
    utxo_address: &T,
    jormungandr_rest_address: &str,
) -> UTxOInfo {
    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);
    utxos
        .into_iter()
        .find(|x| x.address().to_string() == utxo_address.get_address())
        .expect(&format!(
            "None utxo record found for {} of type({})",
            &utxo_address.get_address(),
            &utxo_address.get_address_type()
        ))
}

pub fn assert_node_is_up(address: &str) {
    jcli_wrapper::assert_rest_stats(&address);
}
