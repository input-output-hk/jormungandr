extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};
use super::super::file_utils;
use jormungandr_lib::crypto::hash::Hash;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Witness {
    pub block_hash: Hash,
    pub transaction_id: Hash,
    pub addr_type: String,
    pub private_key_path: PathBuf,
    pub spending_account_counter: Option<u64>,
    pub file: PathBuf,
}

impl Witness {
    pub fn new(
        block_hash: &Hash,
        transaction_id: &Hash,
        addr_type: &str,
        private_key: &str,
        spending_account_counter: Option<u64>,
    ) -> Witness {
        let temp_folder_path = file_utils::get_temp_folder();
        Witness {
            block_hash: block_hash.clone(),
            transaction_id: transaction_id.clone(),
            addr_type: addr_type.to_string(),
            private_key_path: Witness::save_witness_key_temp_file(&temp_folder_path, private_key),
            file: Witness::generate_new_random_witness_file_path(&temp_folder_path),
            spending_account_counter: spending_account_counter,
        }
    }

    pub fn generate_new_random_witness_file_path(temp_folder_path: &PathBuf) -> PathBuf {
        let mut witness_file_path = temp_folder_path.clone();
        witness_file_path.push("witness");
        witness_file_path
    }

    pub fn save_witness_key_temp_file(temp_folder_path: &PathBuf, witness_key: &str) -> PathBuf {
        let witness_key_file = Witness::generate_new_random_witness_key_file_path(temp_folder_path);
        file_utils::create_file_with_content(&witness_key_file, &witness_key);
        println!("Witness key saved into: {:?}", &witness_key);
        witness_key_file
    }

    pub fn generate_new_random_witness_key_file_path(temp_folder_path: &PathBuf) -> PathBuf {
        let mut witness_key_path = temp_folder_path.clone();
        witness_key_path.push("witness_key.secret");
        witness_key_path
    }
}
