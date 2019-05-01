#![allow(dead_code)]

pub mod jcli_transaction_commands;

use self::jcli_transaction_commands::TransactionCommands;
use common::data::witness::Witness;
use common::file_utils;
use common::jcli_wrapper;
use common::process_assert;
use common::process_utils;
use common::process_utils::output_extensions::ProcessOutput;
use std::path::PathBuf;

#[derive(Debug)]
pub struct JCLITransactionWrapper {
    staging_file_path: PathBuf,
    commands: TransactionCommands,
    sealed_transaction_id: String,
}

impl JCLITransactionWrapper {
    pub fn new() -> JCLITransactionWrapper {
        JCLITransactionWrapper {
            staging_file_path: PathBuf::from(""),
            sealed_transaction_id: String::from(""),
            commands: TransactionCommands::new(),
        }
    }

    pub fn new_transaction() -> JCLITransactionWrapper {
        let mut transaction_wraper = JCLITransactionWrapper::new();
        transaction_wraper.assert_new_transaction();
        transaction_wraper
    }

    pub fn assert_new_transaction<'a>(&'a mut self) -> &'a mut JCLITransactionWrapper {
        println!("Running transaction new command...");
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
        tx_id: &str,
        tx_index: &i32,
        amount: &i32,
    ) -> &'a mut JCLITransactionWrapper {
        println!("Running transaction add input command...");
        let output =
            process_utils::run_process_and_get_output(self.commands.get_add_input_command(
                &tx_id,
                &tx_index,
                &amount,
                &self.staging_file_path,
            ));
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_add_output<'a>(
        &'a mut self,
        addr: &str,
        amount: &i32,
    ) -> &'a mut JCLITransactionWrapper {
        println!("Runing add transaction output command...");

        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_add_output_command(&addr, &amount, &self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_finalize<'a>(&'a mut self) -> &'a mut JCLITransactionWrapper {
        println!("Runing finalize transaction command...");

        let output = process_utils::run_process_and_get_output(
            self.commands.get_finalize_command(&self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_finalize_fail(&self, expected_part: &str) -> () {
        println!("Runing finalize transaction command...");

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

    pub fn seal_with_witness<'a>(
        &'a mut self,
        witness: &Witness,
    ) -> &'a mut JCLITransactionWrapper {
        self.assert_make_witness(&witness);
        self.assert_add_witness(&witness);
        self.assert_seal();
        self
    }

    pub fn assert_make_witness<'a>(
        &'a mut self,
        witness: &Witness,
    ) -> &'a mut JCLITransactionWrapper {
        println!("Runing make transaction witness command...");

        let output =
            process_utils::run_process_and_get_output(self.commands.get_make_witness_command(
                &witness.block_hash,
                &witness.transaction_id,
                &witness.addr_type,
                &witness.spending_account_counter,
                &witness.file,
                &witness.private_key_path,
            ));
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn create_witness_from_key(
        &self,
        private_key: &str,
        addr_type: &str,
        jormungandr_rest_address: &str,
    ) -> Witness {
        let transaction_id = self.get_transaction_id();
        let block_hash = jcli_wrapper::assert_rest_get_block_tip(&jormungandr_rest_address);
        let witness = Witness::new(&block_hash, &transaction_id, &addr_type, private_key, &0);
        witness
    }

    pub fn create_witness_default(
        &self,
        addr_type: &str,
        jormungandr_rest_address: &str,
    ) -> Witness {
        let private_key = jcli_wrapper::assert_key_generate_default();
        let transaction_id = self.get_transaction_id();
        let block_hash = jcli_wrapper::assert_rest_get_block_tip(&jormungandr_rest_address);
        let witness = Witness::new(&block_hash, &transaction_id, addr_type, &private_key, &0);
        witness
    }

    pub fn assert_add_witness_fail<'a>(&'a mut self, witness: &Witness, expected_part: &str) -> () {
        println!("Runing add transaction witness command...");

        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_add_witness_command(&witness.file, &self.staging_file_path),
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

    pub fn assert_add_witness<'a>(
        &'a mut self,
        witness: &Witness,
    ) -> &'a mut JCLITransactionWrapper {
        println!("Runing add transaction witness command...");

        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_add_witness_command(&witness.file, &self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_seal<'a>(&'a mut self) -> &'a mut JCLITransactionWrapper {
        println!("Runing seal transaction witness command...");

        let output = process_utils::run_process_and_get_output(
            self.commands.get_seal_command(&self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_transaction_to_message(&self) -> String {
        println!("Runing transaction to message command...");

        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_transaction_message_to_command(&self.staging_file_path),
        );
        let content = output.as_single_line();
        process_assert::assert_process_exited_successfully(output);
        content
    }

    pub fn get_transaction_id(&self) -> String {
        println!("Runing get transaction id command...");

        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_transaction_id_command(&self.staging_file_path),
        );
        let content = output.as_single_line();
        let mut split = content.split_whitespace();
        split.next().unwrap().to_string()
    }
}
