use chain_impl_mockchain::account::SpendingCounter;
use jormungandr_lib::crypto::hash::Hash;

use assert_fs::fixture::PathChild;
use assert_fs::prelude::*;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Copy, Clone)]
pub enum WitnessType {
    Account,
    UTxO,
    //needed for negative testing
    Unknown,
}

impl fmt::Display for WitnessType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Account => write!(f, "account"),
            Self::UTxO => write!(f, "utxo"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug)]
pub struct Witness {
    pub block_hash: Hash,
    pub transaction_id: Hash,
    pub addr_type: WitnessType,
    pub private_key_path: PathBuf,
    pub account_spending_counter: Option<SpendingCounter>,
    pub file: PathBuf,
}

impl Witness {
    pub fn new(
        temp_dir: &impl PathChild,
        block_hash: &Hash,
        transaction_id: &Hash,
        addr_type: WitnessType,
        private_key: &str,
        account_spending_counter: Option<SpendingCounter>,
    ) -> Witness {
        Witness {
            block_hash: *block_hash,
            transaction_id: *transaction_id,
            addr_type,
            private_key_path: write_witness_key(temp_dir, private_key),
            file: temp_dir.child("witness").path().into(),
            account_spending_counter,
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
