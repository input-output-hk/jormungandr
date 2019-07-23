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
    pub fn empty() -> Self {
        JCLITransactionWrapper::new("")
    }

    pub fn new(genesis_hash: &str) -> Self {
        JCLITransactionWrapper {
            staging_file_path: PathBuf::from(""),
            commands: TransactionCommands::new(),
            genesis_hash: Hash::from_hex(&genesis_hash.to_string()).unwrap(),
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
    ) -> JCLITransactionWrapper {
        let mut transaction_builder = JCLITransactionWrapper::new_transaction(genesis_hash);
        transaction_builder
            .assert_add_input(
                &utxo.transaction_id(),
                utxo.index_in_transaction(),
                input_amount,
            )
            .assert_add_output(&receiver.get_address(), output_amount)
            .assert_finalize()
            .seal_with_witness_default(&sender.get_private_key(), &receiver.get_address_type());
        transaction_builder
    }

    pub fn build_transaction<T: AddressDataProvider, U: AddressDataProvider>(
        transaction_id: &Hash,
        transaction_index: u8,
        input_amount: &Value,
        receiver: &T,
        output_amount: &Value,
        sender: &U,
        genesis_hash: &str,
    ) -> JCLITransactionWrapper {
        let mut transaction_builder = JCLITransactionWrapper::new_transaction(genesis_hash);
        transaction_builder
            .assert_add_input(transaction_id, transaction_index, input_amount)
            .assert_add_output(&receiver.get_address(), &output_amount)
            .assert_finalize()
            .seal_with_witness_default(&sender.get_private_key(), &receiver.get_address_type());
        transaction_builder
    }

    pub fn assert_new_transaction<'a>(&'a mut self) -> &'a mut JCLITransactionWrapper {
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

    pub fn assert_add_input<'a>(
        &'a mut self,
        tx_id: &Hash,
        tx_index: u8,
        amount: &Value,
    ) -> &'a mut JCLITransactionWrapper {
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

    pub fn assert_add_input_fail<'a>(
        &'a mut self,
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

    pub fn assert_add_input_from_utxo_with_value<'a>(
        &'a mut self,
        utxo: &UTxOInfo,
        amount: &Value,
    ) -> &'a mut JCLITransactionWrapper {
        self.assert_add_input(&utxo.transaction_id(), utxo.index_in_transaction(), &amount)
    }

    pub fn assert_add_input_from_utxo<'a>(
        &'a mut self,
        utxo: &UTxOInfo,
    ) -> &'a mut JCLITransactionWrapper {
        self.assert_add_input(
            &utxo.transaction_id(),
            utxo.index_in_transaction(),
            utxo.associated_fund(),
        )
    }

    pub fn assert_add_account<'a>(
        &'a mut self,
        account_addr: &str,
        amount: &Value,
    ) -> &'a mut JCLITransactionWrapper {
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

    pub fn assert_add_account_from_legacy<'a>(
        &'a mut self,
        fund: &Fund,
    ) -> &'a mut JCLITransactionWrapper {
        self.assert_add_account(&fund.address, &fund.value)
    }

    pub fn assert_add_output<'a>(
        &'a mut self,
        addr: &str,
        amount: &Value,
    ) -> &'a mut JCLITransactionWrapper {
        let output =
            process_utils::run_process_and_get_output(self.commands.get_add_output_command(
                &addr,
                &amount.to_string(),
                &self.staging_file_path,
            ));
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_finalize<'a>(&'a mut self) -> &'a mut JCLITransactionWrapper {
        let output = process_utils::run_process_and_get_output(
            self.commands.get_finalize_command(&self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_finalize_with_fee<'a>(
        &'a mut self,
        address: &str,
        linear_fee: &LinearFees,
    ) -> &'a mut JCLITransactionWrapper {
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

    pub fn make_and_add_witness_default<'a>(
        &'a mut self,
        private_key: &str,
        transaction_type: &str,
    ) -> &'a mut JCLITransactionWrapper {
        let witness = self.create_witness_from_key(&private_key, &transaction_type);
        self.assert_make_witness(&witness);
        self.assert_add_witness(&witness);
        self
    }

    pub fn seal_with_witness_for_address<'a, T: AddressDataProvider>(
        &'a mut self,
        address: &T,
    ) -> &'a mut JCLITransactionWrapper {
        self.seal_with_witness_default(&address.get_private_key(), &address.get_address_type())
    }

    pub fn seal_with_witness_default<'a>(
        &'a mut self,
        private_key: &str,
        transaction_type: &str,
    ) -> &'a mut JCLITransactionWrapper {
        let witness = self.create_witness_from_key(&private_key, &transaction_type);
        self.seal_with_witness(&witness);
        self
    }

    pub fn seal_with_witness<'a>(
        &'a mut self,
        witness: &Witness,
    ) -> &'a mut JCLITransactionWrapper {
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
                &witness.spending_account_counter,
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
                &witness.spending_account_counter,
                &witness.file,
                &witness.private_key_path,
            ),
            &expected_msg,
        );
    }

    pub fn create_witness_from_key(&self, private_key: &str, addr_type: &str) -> Witness {
        let transaction_id = self.get_transaction_id();
        let witness = Witness::new(
            &self.genesis_hash,
            &transaction_id,
            &addr_type,
            private_key,
            &0,
        );
        witness
    }

    pub fn create_witness_default(&self, addr_type: &str) -> Witness {
        let private_key = jcli_wrapper::assert_key_generate_default();
        self.create_witness_from_key(&private_key, &addr_type)
    }

    pub fn assert_add_witness_fail<'a>(&'a mut self, witness: &Witness, expected_part: &str) -> () {
        process_assert::assert_process_failed_and_matches_message(
            self.commands
                .get_add_witness_command(&witness.file, &self.staging_file_path),
            expected_part,
        );
    }

    pub fn assert_add_witness<'a>(
        &'a mut self,
        witness: &Witness,
    ) -> &'a mut JCLITransactionWrapper {
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_add_witness_command(&witness.file, &self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_seal<'a>(&'a mut self) -> &'a mut JCLITransactionWrapper {
        let output = process_utils::run_process_and_get_output(
            self.commands.get_seal_command(&self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_transaction_to_message(&self) -> String {
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_transaction_message_to_command(&self.staging_file_path),
        );
        let content = output.as_single_line();
        process_assert::assert_process_exited_successfully(output);
        content
    }

    pub fn assert_transaction_to_message_fails(&self, expected_msg: &str) {
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
        let content = output.as_single_line();
        let mut split = content.split_whitespace();
        split.next().unwrap().to_string()
    }
}
