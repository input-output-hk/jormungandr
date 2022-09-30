use crate::{
    jcli::{command::TransactionCommand, data::Witness, WitnessData},
    testing::process::ProcessOutput,
};
use assert_cmd::assert::OutputAssertExt;
use assert_fs::TempDir;
use chain_core::{packer::Codec, property::DeserializeFromSlice};
use chain_impl_mockchain::{fee::LinearFee, fragment::Fragment};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{BlockDate, LegacyUTxO, UTxOInfo, Value},
};
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
                witness.addr_type,
                witness.account_spending_counter,
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
                witness.addr_type,
                witness.account_spending_counter,
                &witness.file,
                &witness.private_key_path,
            )
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn create_witness<P: AsRef<Path>>(
        self,
        staging_dir: &TempDir,
        genesis_hash: Hash,
        witness_data: WitnessData,
        staging_file: P,
    ) -> Witness {
        let transaction_id = self.id(staging_file);
        witness_data.into_witness(staging_dir, &genesis_hash, &transaction_id)
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
        valid_until: BlockDate,
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
                valid_until,
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
        Fragment::deserialize_from_slice(&mut Codec::new(fragment_bytes.as_slice()))
            .expect("Failed to parse message")
            .hash()
            .into()
    }

    pub fn set_expiry_date<P: AsRef<Path>>(self, valid_until: BlockDate, staging_file: P) {
        self.command
            .set_expiry_date(&valid_until.to_string(), staging_file)
            .build()
            .assert()
            .success();
    }
}
