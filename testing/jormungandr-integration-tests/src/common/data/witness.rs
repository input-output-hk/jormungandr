use jormungandr_lib::crypto::hash::Hash;
use serde_derive::{Deserialize, Serialize};

use assert_fs::fixture::PathChild;
use assert_fs::prelude::*;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Witness {
    pub block_hash: Hash,
    pub transaction_id: Hash,
    pub addr_type: String,
    pub private_key_path: PathBuf,
    pub spending_account_counter: Option<u32>,
    pub file: PathBuf,
}

impl Witness {
    pub fn new(
        temp_dir: &impl PathChild,
        block_hash: &Hash,
        transaction_id: &Hash,
        addr_type: &str,
        private_key: &str,
        spending_account_counter: Option<u32>,
    ) -> Witness {
        Witness {
            block_hash: *block_hash,
            transaction_id: *transaction_id,
            addr_type: addr_type.to_string(),
            private_key_path: write_witness_key(temp_dir, private_key),
            file: temp_dir.child("witness").path().into(),
            spending_account_counter,
        }
    }
}

fn write_witness_key(temp_dir: &impl PathChild, witness_key: &str) -> PathBuf {
    let file = temp_dir.child("witness_key.secret");
    file.write_str(witness_key).unwrap();
    let path = file.path().to_path_buf();
    println!("Witness key saved into: {:?}", path);
    path
}
