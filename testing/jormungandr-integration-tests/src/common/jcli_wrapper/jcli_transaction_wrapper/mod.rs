#![allow(dead_code)]

pub mod jcli_transaction_commands;

use self::jcli_transaction_commands::TransactionCommands;
use crate::common::{
    data::witness::Witness, jcli_wrapper, process_utils::output_extensions::ProcessOutput,
};
use assert_cmd::assert::OutputAssertExt;
use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use chain_core::property::Deserialize;
use chain_impl_mockchain::{fee::LinearFee, fragment::Fragment};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{LegacyUTxO, UTxOInfo, Value},
};
use jormungandr_testing_utils::wallet::Wallet;
use std::path::{Path, PathBuf};

pub struct JCLITransactionWrapper {
    staging_dir: TempDir,
    commands: TransactionCommands,
    pub genesis_hash: Hash,
}

impl JCLITransactionWrapper {
    pub fn new(genesis_hash: &str) -> Self {
        let staging_dir = TempDir::new().unwrap();
        JCLITransactionWrapper {
            staging_dir,
            commands: TransactionCommands::new(),
            genesis_hash: Hash::from_hex(genesis_hash).unwrap(),
        }
    }

    fn staging_file(&self) -> ChildPath {
        self.staging_dir.child("transaction.tx")
    }

    pub fn staging_file_path(&self) -> PathBuf {
        PathBuf::from(self.staging_file().path())
    }

    pub fn new_transaction(genesis_hash: &str) -> Self {
        let mut transaction_builder = JCLITransactionWrapper::new(genesis_hash);
        transaction_builder.assert_new_transaction();
        transaction_builder
    }

    pub fn build_transaction_from_utxo(
        utxo: &UTxOInfo,
        input_amount: Value,
        receiver: &Wallet,
        output_amount: Value,
        sender: &Wallet,
        genesis_hash: &str,
    ) -> String {
        JCLITransactionWrapper::new_transaction(genesis_hash)
            .assert_add_input(
                &utxo.transaction_id(),
                utxo.index_in_transaction(),
                input_amount,
            )
            .assert_add_output(&receiver.address().to_string(), output_amount)
            .assert_finalize()
            .seal_with_witness_for_address(&sender)
            .assert_to_message()
    }

    pub fn build_transaction(
        transaction_id: &Hash,
        transaction_index: u8,
        input_amount: Value,
        receiver: &Wallet,
        output_amount: Value,
        sender: &Wallet,
        genesis_hash: &str,
    ) -> String {
        JCLITransactionWrapper::new_transaction(genesis_hash)
            .assert_add_input(transaction_id, transaction_index, input_amount)
            .assert_add_output(&receiver.address().to_string(), output_amount)
            .assert_finalize()
            .seal_with_witness_for_address(&sender)
            .assert_to_message()
    }

    pub fn assert_new_transaction(&mut self) -> &mut Self {
        self.reset_staging_dir();
        self.commands
            .get_new_transaction_command(self.staging_file().path())
            .assert()
            .success();
        self
    }

    fn reset_staging_dir(&mut self) {
        self.staging_dir = TempDir::new().unwrap();
    }

    pub fn assert_add_input(&mut self, tx_id: &Hash, tx_index: u8, amount: Value) -> &mut Self {
        self.commands
            .get_add_input_command(
                &tx_id.to_hex(),
                tx_index,
                &amount.to_string(),
                self.staging_file().path(),
            )
            .assert()
            .success();
        self
    }

    pub fn assert_add_input_fail(
        &mut self,
        tx_id: &Hash,
        tx_index: u8,
        amount: &str,
        expected_part: &str,
    ) {
        self.commands
            .get_add_input_command(
                &tx_id.to_hex(),
                tx_index,
                amount,
                self.staging_file().path(),
            )
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_part));
    }

    pub fn assert_add_input_from_utxo_with_value(
        &mut self,
        utxo: &UTxOInfo,
        amount: Value,
    ) -> &mut Self {
        self.assert_add_input(&utxo.transaction_id(), utxo.index_in_transaction(), amount)
    }

    pub fn assert_add_input_from_utxo(&mut self, utxo: &UTxOInfo) -> &mut Self {
        self.assert_add_input(
            &utxo.transaction_id(),
            utxo.index_in_transaction(),
            *utxo.associated_fund(),
        )
    }

    pub fn assert_add_certificate(&mut self, certificate: &str) -> &mut Self {
        self.commands
            .get_add_certificate_command(&certificate, self.staging_file().path())
            .assert()
            .success();
        self
    }

    pub fn assert_add_account(&mut self, account_addr: &str, amount: &Value) -> &mut Self {
        self.commands
            .get_add_account_command(
                &account_addr,
                &amount.to_string(),
                self.staging_file().path(),
            )
            .assert()
            .success();
        self
    }

    pub fn assert_add_account_fail(&self, account_addr: &str, amount: Value, expected_msg: &str) {
        self.commands
            .get_add_account_command(
                &account_addr,
                &amount.to_string(),
                self.staging_file().path(),
            )
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn assert_add_account_from_legacy(&mut self, fund: &LegacyUTxO) -> &mut Self {
        self.assert_add_account(&fund.address.to_string(), &fund.value)
    }

    pub fn assert_add_output(&mut self, addr: &str, amount: Value) -> &mut Self {
        self.commands
            .get_add_output_command(&addr, &amount.to_string(), self.staging_file().path())
            .assert()
            .success();
        self
    }

    pub fn assert_finalize(&mut self) -> &mut Self {
        self.commands
            .get_finalize_command(self.staging_file().path())
            .assert()
            .success();
        self
    }

    pub fn assert_finalize_with_fee(&mut self, address: &str, linear_fee: &LinearFee) -> &mut Self {
        self.commands
            .get_finalize_with_fee_command(&address, &linear_fee, self.staging_file().path())
            .assert()
            .success();
        self
    }

    pub fn assert_finalize_fail(&self, expected_part: &str) {
        self.commands
            .get_finalize_command(self.staging_file().path())
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_part));
    }

    pub fn assert_add_auth(&mut self, key: &Path) -> &mut Self {
        self.commands
            .get_auth_command(key, self.staging_file().path())
            .assert()
            .success();
        self
    }

    pub fn make_and_add_witness_default(&mut self, wallet: &Wallet) -> &mut Self {
        let witness = self.create_witness_from_wallet(&wallet);
        self.assert_make_witness(&witness);
        self.assert_add_witness(&witness);
        self
    }

    pub fn seal_with_witness_for_address(&mut self, wallet: &Wallet) -> &mut Self {
        let witness = self.create_witness_from_wallet(&wallet);
        self.seal_with_witness(&witness);
        self
    }

    pub fn seal_with_witness_default(
        &mut self,
        private_key: &str,
        transaction_type: &str,
        spending_key: Option<u32>,
    ) -> &mut Self {
        let witness = self.create_witness_from_key(&private_key, &transaction_type, spending_key);
        self.seal_with_witness(&witness);
        self
    }

    pub fn seal_with_witness(&mut self, witness: &Witness) -> &mut Self {
        self.assert_make_witness(&witness);
        self.assert_add_witness(&witness);
        self.assert_seal();
        self
    }

    pub fn assert_make_witness(&mut self, witness: &Witness) -> &mut Self {
        self.commands
            .get_make_witness_command(
                &witness.block_hash.to_hex(),
                &witness.transaction_id.to_hex(),
                &witness.addr_type,
                witness.spending_account_counter,
                &witness.file,
                &witness.private_key_path,
            )
            .assert()
            .success();
        self
    }

    pub fn assert_make_witness_fails(&self, witness: &Witness, expected_msg: &str) {
        self.commands
            .get_make_witness_command(
                &witness.block_hash.to_hex(),
                &witness.transaction_id.to_hex(),
                &witness.addr_type,
                witness.spending_account_counter,
                &witness.file,
                &witness.private_key_path,
            )
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn create_witness_from_wallet(&self, wallet: &Wallet) -> Witness {
        match wallet {
            Wallet::Account(account) => self.create_witness_from_key(
                &account.signing_key().to_bech32_str(),
                &"account",
                Some(account.internal_counter().into()),
            ),
            Wallet::UTxO(utxo) => self.create_witness_from_key(
                &utxo.last_signing_key().to_bech32_str(),
                &"utxo",
                None,
            ),
            Wallet::Delegation(delegation) => self.create_witness_from_key(
                &delegation.last_signing_key().to_bech32_str(),
                &"utxo",
                None,
            ),
        }
    }

    pub fn create_witness_from_key(
        &self,
        private_key: &str,
        addr_type: &str,
        spending_key: Option<u32>,
    ) -> Witness {
        let transaction_id = self.get_transaction_id();
        Witness::new(
            &self.staging_dir,
            &self.genesis_hash,
            &transaction_id,
            &addr_type,
            private_key,
            spending_key,
        )
    }

    pub fn create_witness_default(&self, addr_type: &str, spending_key: Option<u32>) -> Witness {
        let private_key = jcli_wrapper::assert_key_generate_default();
        self.create_witness_from_key(&private_key, &addr_type, spending_key)
    }

    pub fn assert_add_witness_fail(&mut self, witness: &Witness, expected_part: &str) {
        self.commands
            .get_add_witness_command(&witness.file, self.staging_file().path())
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_part));
    }

    pub fn assert_add_witness(&mut self, witness: &Witness) -> &mut Self {
        self.commands
            .get_add_witness_command(&witness.file, self.staging_file().path())
            .assert()
            .success();
        self
    }

    pub fn assert_seal(&mut self) -> &mut Self {
        self.commands
            .get_seal_command(self.staging_file().path())
            .assert()
            .success();
        self
    }

    pub fn assert_to_message(&self) -> String {
        self.commands
            .get_transaction_message_to_command(self.staging_file().path())
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn assert_to_message_fails(&self, expected_msg: &str) {
        self.commands
            .get_transaction_message_to_command(self.staging_file().path())
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn get_transaction_id(&self) -> Hash {
        self.commands
            .get_transaction_id_command(self.staging_file().path())
            .assert()
            .success()
            .get_output()
            .as_hash()
    }

    pub fn get_transaction_info(&self, format: &str) -> String {
        self.commands
            .get_transaction_info_command(&format, self.staging_file().path())
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn get_fragment_id(&self) -> Hash {
        let fragment_hex = self.assert_to_message();
        let fragment_bytes = hex::decode(&fragment_hex).expect("Failed to parse message hex");
        Fragment::deserialize(fragment_bytes.as_slice())
            .expect("Failed to parse message")
            .hash()
            .into()
    }
}
