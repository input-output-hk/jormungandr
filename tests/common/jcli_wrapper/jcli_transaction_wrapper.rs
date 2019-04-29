use super::configuration;
use super::file_utils;
use super::process_assert;
use super::process_utils;
use super::process_utils::output_extensions::ProcessOutput;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub struct JCLITransactionWrapper {
    pub witness_key: PathBuf,

    staging_file: PathBuf,
    witness_file: PathBuf,
    sealed_transaction_id: String,
}

impl JCLITransactionWrapper {
    pub fn new() -> JCLITransactionWrapper {
        let temp_folder = file_utils::get_temp_folder();

        let mut staging_file = temp_folder.clone();
        staging_file.push("transaction.tx");

        let mut witness_key = temp_folder.clone();
        witness_key.push("witness_key.secret");

        let mut witness_file = temp_folder.clone();
        witness_file.push("witness");

        JCLITransactionWrapper {
            staging_file,
            witness_key,
            witness_file,
            sealed_transaction_id: String::from(""),
        }
    }

    pub fn assert_new_transaction(&self) -> () {
        println!("Running transaction new command...");
        let output = process_utils::run_process_and_get_output(self.get_new_transaction_command());
        process_assert::assert_process_exited_successfully(output);
    }

    fn get_new_transaction_command(&self) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("transaction")
            .arg("new")
            .arg("--staging")
            .arg(&self.staging_file.as_os_str());
        command
    }

    pub fn assert_add_input(&self, tx_id: &str, tx_index: &i32, amount: &i32) -> () {
        println!("Running transaction add input command...");
        let output = process_utils::run_process_and_get_output(
            self.get_add_input_command(&tx_id, &tx_index, &amount),
        );
        process_assert::assert_process_exited_successfully(output);
    }

    fn get_add_input_command(&self, tx_id: &str, tx_index: &i32, amount: &i32) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("transaction")
            .arg("add-input")
            .arg(&tx_id)
            .arg(tx_index.to_string())
            .arg(amount.to_string())
            .arg("--staging")
            .arg(&self.staging_file.as_os_str());
        command
    }

    pub fn assert_add_output(&self, addr: &str, amount: &i32) -> () {
        println!("Runing add transaction output command...");

        let output =
            process_utils::run_process_and_get_output(self.get_add_output_command(&addr, &amount));
        process_assert::assert_process_exited_successfully(output);
    }

    fn get_add_output_command(&self, addr: &str, amount: &i32) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("transaction")
            .arg("add-output")
            .arg(&addr)
            .arg(amount.to_string())
            .arg("--staging")
            .arg(&self.staging_file.as_os_str());
        command
    }

    pub fn assert_finalize(&self) -> () {
        println!("Runing finalize transaction command...");

        let output = process_utils::run_process_and_get_output(self.get_finalize_command());
        process_assert::assert_process_exited_successfully(output);
    }

    pub fn assert_finalize_fail(&self, expected_part: &str) -> () {
        println!("Runing finalize transaction command...");

        let output = process_utils::run_process_and_get_output(self.get_finalize_command());
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

    fn get_finalize_command(&self) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("transaction")
            .arg("finalize")
            .arg("--staging")
            .arg(&self.staging_file.as_os_str());
        command
    }

    pub fn assert_make_witness(
        &self,
        block0_hash: &str,
        tx_id: &str,
        addr_type: &str,
        spending_account_counter: &i32,
    ) -> () {
        println!("Runing make transaction witness command...");

        let output = process_utils::run_process_and_get_output(self.get_make_witness_command(
            block0_hash,
            &tx_id,
            &addr_type,
            &spending_account_counter,
        ));
        process_assert::assert_process_exited_successfully(output);
    }

    fn get_make_witness_command(
        &self,
        block0_hash: &str,
        tx_id: &str,
        addr_type: &str,
        spending_account_counter: &i32,
    ) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("transaction")
            .arg("make-witness")
            .arg("--genesis-block-hash")
            .arg(block0_hash)
            .arg("--type")
            .arg(&addr_type)
            .arg(&tx_id)
            .arg(self.witness_file.as_os_str())
            .arg(spending_account_counter.to_string())
            .arg(self.witness_key.as_os_str());
        command
    }

    pub fn assert_add_witness_fail(&self, expected_part: &str) -> () {
        println!("Runing add transaction witness command...");

        let output = process_utils::run_process_and_get_output(self.get_add_witness_command());
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

    pub fn assert_add_witness(&self) -> () {
        println!("Runing add transaction witness command...");

        let output = process_utils::run_process_and_get_output(self.get_add_witness_command());
        process_assert::assert_process_exited_successfully(output);
    }

    fn get_add_witness_command(&self) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("transaction")
            .arg("add-witness")
            .arg(self.witness_file.as_os_str())
            .arg("--staging")
            .arg(self.staging_file.as_os_str());
        command
    }

    pub fn assert_seal(&self) -> () {
        println!("Runing seal transaction witness command...");

        let output = process_utils::run_process_and_get_output(self.get_seal_command());
        process_assert::assert_process_exited_successfully(output);
    }

    fn get_seal_command(&self) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("transaction")
            .arg("seal")
            .arg("--staging")
            .arg(&self.staging_file.as_os_str());
        command
    }

    pub fn assert_transaction_to_message(&self) -> String {
        println!("Runing transaction to message command...");

        let output =
            process_utils::run_process_and_get_output(self.get_transaction_message_to_command());
        let content = output.as_single_line();
        process_assert::assert_process_exited_successfully(output);
        content
    }

    fn get_transaction_message_to_command(&self) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("transaction")
            .arg("to-message")
            .arg("--staging")
            .arg(&self.staging_file.as_os_str());
        command
    }

    pub fn save_witness_key(&self, content: &str) -> () {
        file_utils::create_file_with_content(&self.witness_key, &content);
        println!("Witness key saved into: {:?}", &self.witness_key);
    }

    pub fn get_transaction_id(&self) -> String {
        println!("Runing get transaction id command...");

        let output = process_utils::run_process_and_get_output(self.get_transaction_id_command());
        let content = output.as_single_line();
        let mut split = content.split_whitespace();
        split.next().unwrap().to_string()
    }

    fn get_transaction_id_command(&self) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("transaction")
            .arg("info")
            .arg("--format")
            .arg("{id}")
            .arg("--staging")
            .arg(&self.staging_file.as_os_str());
        command
    }
}
