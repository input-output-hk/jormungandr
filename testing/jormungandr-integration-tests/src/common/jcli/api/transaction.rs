use crate::common::{data::witness::Witness, jcli::command::TransactionCommand};
use assert_cmd::assert::OutputAssertExt;
use assert_fs::TempDir;
use chain_core::property::Deserialize;
use chain_impl_mockchain::{fee::LinearFee, fragment::Fragment};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{LegacyUTxO, UTxOInfo, Value},
};
use jormungandr_testing_utils::testing::process::ProcessOutput;
use jormungandr_testing_utils::wallet::Wallet;
use jortestkit::process::output_extensions::ProcessOutput as _;
use std::path::Path;

pub struct Transaction {
    command: TransactionCommand,
}

impl Transaction {
    pub fn new(command: TransactionCommand) -> Self {
        Self { command }
    }

    pub fn new_transaction<P: AsRef<Path>>(self, staging_file: P) {
        self.command
            .new_transaction(staging_file)
            .build()
            .assert()
            .success();
    }

    pub fn add_input<P: AsRef<Path>>(
        self,
        tx_id: &Hash,
        tx_index: u8,
        amount: &str,
        staging_file: P,
    ) {
        self.command
            .add_input(&tx_id.to_hex(), tx_index, amount, staging_file)
            .build()
            .assert()
            .success();
    }

    pub fn add_input_expect_fail<P: AsRef<Path>>(
        self,
        tx_id: &Hash,
        tx_index: u8,
        amount: &str,
        staging_file: P,
        expected_part: &str,
    ) {
        self.command
            .add_input(&tx_id.to_hex(), tx_index, amount, staging_file)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_part));
    }

    pub fn add_input_from_utxo_with_value<P: AsRef<Path>>(
        self,
        utxo: &UTxOInfo,
        amount: Value,
        staging_file: P,
    ) {
        self.add_input(
            utxo.transaction_id(),
            utxo.index_in_transaction(),
            &amount.to_string(),
            staging_file,
        )
    }

    pub fn add_input_from_utxo<P: AsRef<Path>>(self, utxo: &UTxOInfo, staging_file: P) {
        self.add_input(
            utxo.transaction_id(),
            utxo.index_in_transaction(),
            &utxo.associated_fund().to_string(),
            staging_file,
        );
    }

    pub fn add_certificate<S: Into<String>, P: AsRef<Path>>(self, certificate: S, staging_file: P) {
        self.command
            .add_certificate(certificate, staging_file)
            .build()
            .assert()
            .success();
    }

    pub fn add_account<P: AsRef<Path>>(self, account_addr: &str, amount: &str, staging_file: P) {
        self.command
            .add_account(account_addr, amount, staging_file)
            .build()
            .assert()
            .success();
    }

    pub fn add_account_expect_fail<P: AsRef<Path>>(
        self,
        account_addr: &str,
        amount: &str,
        staging_file: P,
        expected_msg: &str,
    ) {
        self.command
            .add_account(account_addr, amount, staging_file)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn add_account_from_legacy<P: AsRef<Path>>(self, fund: &LegacyUTxO, staging_file: P) {
        self.add_account(
            &fund.address.to_string(),
            &fund.value.to_string(),
            staging_file,
        )
    }

    pub fn add_output<P: AsRef<Path>>(self, addr: &str, amount: Value, staging_file: P) {
        self.command
            .add_output(addr, &amount.to_string(), staging_file)
            .build()
            .assert()
            .success();
    }

    pub fn finalize<P: AsRef<Path>>(self, staging_file: P) {
        self.command
            .finalize(staging_file)
            .build()
            .assert()
            .success();
    }

    pub fn finalize_with_fee<P: AsRef<Path>>(
        self,
        address: &str,
        linear_fee: &LinearFee,
        staging_file: P,
    ) {
        self.command
            .finalize_with_fee(address, linear_fee, staging_file)
            .build()
            .assert()
            .success();
    }

    pub fn finalize_expect_fail<P: AsRef<Path>>(self, staging_file: P, expected_part: &str) {
        self.command
            .finalize(staging_file)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_part));
    }

    pub fn auth<P: AsRef<Path>, Q: AsRef<Path>>(self, key: P, staging_file: Q) {
        self.command
            .auth(key, staging_file)
            .build()
            .assert()
            .success();
    }

    pub fn make_witness(self, witness: &Witness) {
        self.command
            .make_witness(
                &witness.block_hash.to_hex(),
                &witness.transaction_id.to_hex(),
                &witness.addr_type,
                witness.spending_account_counter,
                &witness.file,
                &witness.private_key_path,
            )
            .build()
            .assert()
            .success();
    }

    pub fn make_witness_expect_fail(self, witness: &Witness, expected_msg: &str) {
        self.command
            .make_witness(
                &witness.block_hash.to_hex(),
                &witness.transaction_id.to_hex(),
                &witness.addr_type,
                witness.spending_account_counter,
                &witness.file,
                &witness.private_key_path,
            )
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn create_witness_from_wallet<P: AsRef<Path>>(
        self,
        staging_dir: &TempDir,
        genesis_hash: Hash,
        wallet: &Wallet,
        staging_file: P,
    ) -> Witness {
        match wallet {
            Wallet::Account(account) => self.create_witness_from_key(
                staging_dir,
                genesis_hash,
                &account.signing_key().to_bech32_str(),
                "account",
                Some(account.internal_counter().into()),
                staging_file,
            ),
            Wallet::UTxO(utxo) => self.create_witness_from_key(
                staging_dir,
                genesis_hash,
                &utxo.last_signing_key().to_bech32_str(),
                "utxo",
                None,
                staging_file,
            ),
            Wallet::Delegation(delegation) => self.create_witness_from_key(
                staging_dir,
                genesis_hash,
                &delegation.last_signing_key().to_bech32_str(),
                "utxo",
                None,
                staging_file,
            ),
        }
    }

    pub fn create_witness_from_key<P: AsRef<Path>>(
        self,
        staging_dir: &TempDir,
        genesis_hash: Hash,
        private_key: &str,
        addr_type: &str,
        spending_key: Option<u32>,
        staging_file: P,
    ) -> Witness {
        let transaction_id = self.id(staging_file);
        Witness::new(
            staging_dir,
            &genesis_hash,
            &transaction_id,
            addr_type,
            private_key,
            spending_key,
        )
    }

    pub fn add_witness_expect_fail<P: AsRef<Path>>(
        self,
        witness: &Witness,
        staging_file: P,
        expected_part: &str,
    ) {
        self.command
            .add_witness(&witness.file, staging_file)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_part));
    }

    pub fn add_witness<P: AsRef<Path>>(self, witness: &Witness, staging_file: P) {
        self.command
            .add_witness(&witness.file, staging_file)
            .build()
            .assert()
            .success();
    }

    pub fn seal<P: AsRef<Path>>(self, staging_file: P) {
        self.command.seal(staging_file).build().assert().success();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn make_transaction(
        self,
        host: String,
        sender: jormungandr_lib::interfaces::Address,
        receiver: Option<jormungandr_lib::interfaces::Address>,
        value: jormungandr_lib::interfaces::Value,
        block0_hash: String,
        secret: impl AsRef<Path>,
        staging_file: impl AsRef<Path>,
        post: bool,
    ) {
        self.command
            .make_transaction(
                host,
                sender,
                receiver,
                value,
                block0_hash,
                secret,
                staging_file,
                post,
            )
            .build()
            .assert()
            .success();
    }

    pub fn convert_to_message<P: AsRef<Path>>(self, staging_file: P) -> String {
        self.command
            .to_message(staging_file)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn convert_to_message_expect_fail<P: AsRef<Path>>(
        self,
        staging_file: P,
        expected_msg: &str,
    ) {
        self.command
            .to_message(staging_file)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn id<P: AsRef<Path>>(self, staging_file: P) -> Hash {
        self.command
            .id(staging_file)
            .build()
            .assert()
            .success()
            .get_output()
            .as_hash()
    }

    pub fn info<P: AsRef<Path>>(self, format: &str, staging_file: P) -> String {
        self.command
            .info(format, staging_file)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn fragment_id<P: AsRef<Path>>(self, staging_file: P) -> Hash {
        let fragment_hex = self.convert_to_message(staging_file);
        let fragment_bytes = hex::decode(&fragment_hex).expect("Failed to parse message hex");
        Fragment::deserialize(fragment_bytes.as_slice())
            .expect("Failed to parse message")
            .hash()
            .into()
    }
}
