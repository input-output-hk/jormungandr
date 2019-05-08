#![allow(dead_code)]

pub mod jcli_transaction_commands;

use self::jcli_transaction_commands::TransactionCommands;
use common::configuration::genesis_model::Fund;
use common::data::address::AddressDataProvider;
use common::data::utxo::Utxo as UtxoData;
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
    genesis_hash: String,
}

impl JCLITransactionWrapper {
    pub fn new() -> JCLITransactionWrapper {
        JCLITransactionWrapper::from_genesis("")
    }

    pub fn from_genesis(genesis_hash: &str) -> JCLITransactionWrapper {
        JCLITransactionWrapper {
            staging_file_path: PathBuf::from(""),
            commands: TransactionCommands::new(),
            genesis_hash: genesis_hash.to_string(),
        }
    }

    pub fn new_transaction(genesis_hash: &str) -> JCLITransactionWrapper {
        let mut transaction_builder = JCLITransactionWrapper::from_genesis(genesis_hash);
        transaction_builder.assert_new_transaction();
        transaction_builder
    }

    pub fn build_transaction_from_utxo<T: AddressDataProvider, U: AddressDataProvider>(
        utxo: &UtxoData,
        input_amount: &i32,
        reciever: &T,
        output_amount: &i32,
        sender: &U,
        genesis_hash: &str,
    ) -> JCLITransactionWrapper {
        let mut transaction_builder = JCLITransactionWrapper::new_transaction(genesis_hash);
        transaction_builder
            .assert_add_input(&utxo.in_txid, &utxo.in_idx, &input_amount)
            .assert_add_output(&reciever.get_address(), &output_amount)
            .assert_finalize()
            .seal_with_witness_deafult(&sender.get_private_key(), &reciever.get_address_type());
        transaction_builder
    }

    pub fn build_transaction<T: AddressDataProvider, U: AddressDataProvider>(
        transaction_id: &str,
        transaction_index: &i32,
        input_amount: &i32,
        reciever: &T,
        output_amount: &i32,
        sender: &U,
        genesis_hash: &str,
    ) -> JCLITransactionWrapper {
        let mut transaction_builder = JCLITransactionWrapper::new_transaction(genesis_hash);
        transaction_builder
            .assert_add_input(&transaction_id, &transaction_index, &input_amount)
            .assert_add_output(&reciever.get_address(), &output_amount)
            .assert_finalize()
            .seal_with_witness_deafult(&sender.get_private_key(), &reciever.get_address_type());
        transaction_builder
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
                &amount.to_string(),
                &self.staging_file_path,
            ));
        process_assert::assert_process_exited_successfully(output);
        self
    }

    pub fn assert_add_input_fail<'a>(
        &'a mut self,
        tx_id: &str,
        tx_index: &i32,
        amount: &str,
        expected_part: &str,
    ) -> () {
        println!("Running transaction add input command...");
        process_assert::assert_process_failed_and_contains_message(
            self.commands.get_add_input_command(
                &tx_id,
                &tx_index,
                &amount,
                &self.staging_file_path,
            ),
            expected_part,
        );
    }

    pub fn assert_add_input_from_utxo_with_value<'a>(
        &'a mut self,
        utxo: &UtxoData,
        amount: &i32,
    ) -> &'a mut JCLITransactionWrapper {
        self.assert_add_input(&utxo.in_txid, &utxo.in_idx, &amount)
    }

    pub fn assert_add_input_from_utxo<'a>(
        &'a mut self,
        utxo: &UtxoData,
    ) -> &'a mut JCLITransactionWrapper {
        self.assert_add_input(&utxo.in_txid, &utxo.in_idx, &utxo.out_value)
    }

    pub fn assert_add_account<'a>(
        &'a mut self,
        account_addr: &str,
        amount: &i32,
    ) -> &'a mut JCLITransactionWrapper {
        println!("Running transaction add account command...");
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_add_account_command(&account_addr, &amount, &self.staging_file_path),
        );
        process_assert::assert_process_exited_successfully(output);
        self
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

    pub fn seal_with_witness_deafult<'a>(
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
