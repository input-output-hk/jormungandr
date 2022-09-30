use assert_fs::{fixture::PathChild, prelude::*, TempDir};
use chain_impl_mockchain::account::SpendingCounter;
use jormungandr_lib::crypto::hash::Hash;
use std::{fmt, path::PathBuf};

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

pub struct WitnessData {
    pub secret_bech32: String,
    pub addr_type: WitnessType,
    pub spending_counter: Option<SpendingCounter>,
}

impl WitnessData {
    pub fn new_account(signing_key: &str, spending_counter: SpendingCounter) -> Self {
        Self {
            secret_bech32: signing_key.to_owned(),
            addr_type: WitnessType::Account,
            spending_counter: Some(spending_counter),
        }
    }

    pub fn new_utxo(signing_key: &str) -> Self {
        Self {
            secret_bech32: signing_key.to_owned(),
            addr_type: WitnessType::UTxO,
            spending_counter: None,
        }
    }

    pub fn spending_counter(&self) -> Option<SpendingCounter> {
        self.spending_counter
    }

    pub fn into_witness(
        &self,
        staging_dir: &TempDir,
        genesis_hash: &Hash,
        transaction_id: &Hash,
    ) -> Witness {
        Witness::new(
            staging_dir,
            genesis_hash,
            transaction_id,
            self.addr_type,
            &self.secret_bech32,
            self.spending_counter(),
        )
    }
}
