#![allow(dead_code)]

pub mod jcli_transaction_commands;

use self::jcli_transaction_commands::TransactionCommands;
use crate::common::configuration::genesis_model::{Fund, LinearFees};
use crate::common::data::address::AddressDataProvider;
use crate::common::data::witness::Witness;
use crate::common::file_utils;
use crate::common::jcli_wrapper;
use crate::common::process_assert;
use crate::common::process_utils;
use crate::common::process_utils::output_extensions::ProcessOutput;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{UTxOInfo, Value},
};
use std::path::PathBuf;

#[derive(Debug)]
pub struct JCLITransactionWrapper {
    pub staging_file_path: PathBuf,
    commands: TransactionCommands,
    pub genesis_hash: Hash,
}

impl JCLITransactionWrapper {
    pub fn new(genesis_hash: &str) -> Self {
        JCLITransactionWrapper {
            staging_file_path: PathBuf::from(""),
            commands: TransactionCommands::new(),
            genesis_hash: Hash::from_hex(genesis_hash).unwrap(),
        }
    }

    pub fn new_transaction(genesis_hash: &str) -> Self {
        let mut transaction_builder = JCLITransactionWrapper::new(genesis_hash);
        transaction_builder.assert_new_transaction();
        transaction_builder
    }

    pub fn build_transaction_from_utxo<T: AddressDataProvider, U: AddressDataProvider>(
        utxo: &UTxOInfo,
        input_amount: &Value,
        receiver: &T,
        output_amount: &Value,
        sender: &U,
        genesis_hash: &str,
    ) -> String {
        JCLITransactionWrapper::new_transaction(genesis_hash)
            .assert_add_input(
                &utxo.transaction_id(),
                utxo.index_in_transaction(),
                input_amount,
            )
            .assert_add_output(&receiver.get_address(), output_amount)
            .assert_finalize()
            .seal_with_witness_default(
                &sender.get_private_key(),
                &receiver.get_address_type(),
                sender.get_spending_key(),
            )
            .assert_to_message()
    }

    pub fn build_transaction<T: AddressDataProvider, U: AddressDataProvider>(
        transaction_id: &Hash,
        transaction_index: u8,
        input_amount: &Value,
        receiver: &T,
        output_amount: &Value,
        sender: &U,
        genesis_hash: &str,
    ) -> String {
        JCLITransactionWrapper::new_transaction(genesis_hash)
            .assert_add_input(transaction_id, transaction_index, input_amount)
            .assert_add_output(&receiver.get_address(), &output_amount)
            .assert_finalize()
            .seal_with_witness_default(
                &sender.get_private_key(),
                &receiver.get_address_type(),
                sender.get_spending_key(),
            )
            .assert_to_message()
    }

    pub fn assert_new_transaction(&mut self) -> &mut Self {
        self.generate_new_random_staging_file_path();
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_new_transaction_command(&self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    fn generate_new_random_staging_file_path(&mut self) -> () {
        let mut staging_file_path = file_utils::get_temp_folder().clone();
        staging_file_path.push("transaction.tx");
        self.staging_file_path = staging_file_path;
    }

    pub fn assert_add_input(&mut self, tx_id: &Hash, tx_index: u8, amount: &Value) -> &mut Self {
        let output =
            process_utils::run_process_and_get_output(self.commands.get_add_input_command(
                &tx_id.to_hex(),
                tx_index,
                &amount.to_string(),
                &self.staging_file_path,
            ));
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_add_input_fail(
        &mut self,
        tx_id: &Hash,
        tx_index: u8,
        amount: &str,
        expected_part: &str,
    ) -> () {
        process_assert::assert_process_failed_and_contains_message(
            self.commands.get_add_input_command(
                &tx_id.to_hex(),
                tx_index,
                amount,
                &self.staging_file_path,
            ),
            expected_part,
        );
    }

    pub fn assert_add_input_from_utxo_with_value(
        &mut self,
        utxo: &UTxOInfo,
        amount: &Value,
    ) -> &mut Self {
        self.assert_add_input(&utxo.transaction_id(), utxo.index_in_transaction(), &amount)
    }

    pub fn assert_add_input_from_utxo(&mut self, utxo: &UTxOInfo) -> &mut Self {
        self.assert_add_input(
            &utxo.transaction_id(),
            utxo.index_in_transaction(),
            utxo.associated_fund(),
        )
    }

    pub fn assert_add_certificate(&mut self, certificate: &str) -> &mut Self {
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_add_certificate_command(&certificate, &self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_add_account(&mut self, account_addr: &str, amount: &Value) -> &mut Self {
        let output =
            process_utils::run_process_and_get_output(self.commands.get_add_account_command(
                &account_addr,
                &amount.to_string(),
                &self.staging_file_path,
            ));
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_add_account_fail(&self, account_addr: &str, amount: &Value, expected_msg: &str) {
        process_assert::assert_process_failed_and_matches_message(
            self.commands.get_add_account_command(
                &account_addr,
                &amount.to_string(),
                &self.staging_file_path,
            ),
            expected_msg,
        );
    }

    pub fn assert_add_account_from_legacy(&mut self, fund: &Fund) -> &mut Self {
        self.assert_add_account(&fund.address, &fund.value)
    }

    pub fn assert_add_output(&mut self, addr: &str, amount: &Value) -> &mut Self {
        let output =
            process_utils::run_process_and_get_output(self.commands.get_add_output_command(
                &addr,
                &amount.to_string(),
                &self.staging_file_path,
            ));
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_finalize(&mut self) -> &mut Self {
        let output = process_utils::run_process_and_get_output(
            self.commands.get_finalize_command(&self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_finalize_with_fee(
        &mut self,
        address: &str,
        linear_fee: &LinearFees,
    ) -> &mut Self {
        let output =
            process_utils::run_process_and_get_output(self.commands.get_finalize_with_fee_command(
                &address,
                &linear_fee,
                &self.staging_file_path,
            ));
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_finalize_fail(&self, expected_part: &str) -> () {
        let output = process_utils::run_process_and_get_output(
            self.commands.get_finalize_command(&self.staging_file_path),
        );
        let actual = output.err_as_single_line();

        assert_eq!(
            actual.contains(expected_part),
            true,
            "message : '{}' does not contain expected part '{}'",
            &actual,
            &expected_part
        );

        process_assert::assert_process_failed(output);
    }

    pub fn assert_add_auth(&mut self, key: &PathBuf) -> &mut Self {
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_auth_command(&key, &self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn make_and_add_witness_default(
        &mut self,
        private_key: &str,
        transaction_type: &str,
        spending_key: Option<u64>,
    ) -> &mut Self {
        let witness = self.create_witness_from_key(&private_key, &transaction_type, spending_key);
        self.assert_make_witness(&witness);
        self.assert_add_witness(&witness);
        self
    }

    pub fn seal_with_witness_for_address<'a, T: AddressDataProvider>(
        &mut self,
        address: &T,
    ) -> &mut Self {
        self.seal_with_witness_default(
            &address.get_private_key(),
            &address.get_address_type(),
            address.get_spending_key(),
        )
    }

    pub fn seal_with_witness_default(
        &mut self,
        private_key: &str,
        transaction_type: &str,
        spending_key: Option<u64>,
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
        let output =
            process_utils::run_process_and_get_output(self.commands.get_make_witness_command(
                &witness.block_hash.to_hex(),
                &witness.transaction_id.to_hex(),
                &witness.addr_type,
                witness.spending_account_counter,
                &witness.file,
                &witness.private_key_path,
            ));
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_make_witness_fails(&self, witness: &Witness, expected_msg: &str) {
        process_assert::assert_process_failed_and_matches_message(
            self.commands.get_make_witness_command(
                &witness.block_hash.to_hex(),
                &witness.transaction_id.to_hex(),
                &witness.addr_type,
                witness.spending_account_counter,
                &witness.file,
                &witness.private_key_path,
            ),
            &expected_msg,
        );
    }

    pub fn create_witness_from_key(
        &self,
        private_key: &str,
        addr_type: &str,
        spending_key: Option<u64>,
    ) -> Witness {
        let transaction_id = self.get_transaction_id();
        let witness = Witness::new(
            &self.genesis_hash,
            &transaction_id,
            &addr_type,
            private_key,
            spending_key,
        );
        witness
    }

    pub fn create_witness_default(&self, addr_type: &str, spending_key: Option<u64>) -> Witness {
        let private_key = jcli_wrapper::assert_key_generate_default();
        self.create_witness_from_key(&private_key, &addr_type, spending_key)
    }

    pub fn assert_add_witness_fail(&mut self, witness: &Witness, expected_part: &str) -> () {
        process_assert::assert_process_failed_and_matches_message(
            self.commands
                .get_add_witness_command(&witness.file, &self.staging_file_path),
            expected_part,
        );
    }

    pub fn assert_add_witness(&mut self, witness: &Witness) -> &mut Self {
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_add_witness_command(&witness.file, &self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_seal(&mut self) -> &mut Self {
        let output = process_utils::run_process_and_get_output(
            self.commands.get_seal_command(&self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_to_message(&self) -> String {
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_transaction_message_to_command(&self.staging_file_path),
        );
        let content = output.as_single_line();
        process_assert::assert_process_exited_successfully(output);
        content
    }

    pub fn assert_to_message_fails(&self, expected_msg: &str) {
        process_assert::assert_process_failed_and_matches_message(
            self.commands
                .get_transaction_message_to_command(&self.staging_file_path),
            expected_msg,
        );
    }

    pub fn get_transaction_id(&self) -> Hash {
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_transaction_id_command(&self.staging_file_path),
        );
        Hash::from_hex(output.as_single_line().as_str())
            .expect("Cannot parse transaction id into hash")
    }

    pub fn get_transaction_info(&self, format: &str) -> String {
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_transaction_info_command(&format, &self.staging_file_path),
        );
        output.as_single_line()
    }
}
