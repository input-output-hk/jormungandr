#![allow(dead_code)]

use crate::testing::common::jcli::JCli;
use crate::testing::witness::Witness;
use crate::wallet::Wallet;
use assert_fs::fixture::ChildPath;
use assert_fs::{prelude::*, TempDir};
use chain_core::property::Deserialize;
use chain_impl_mockchain::{fee::LinearFee, fragment::Fragment};
use jormungandr_lib::interfaces::BlockDate;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{LegacyUTxO, UTxOInfo, Value},
};
use std::path::{Path, PathBuf};

pub struct TransactionBuilder {
    staging_dir: TempDir,
    jcli: JCli,
    genesis_hash: Hash,
}

impl TransactionBuilder {
    pub fn new(jcli: JCli, genesis_hash: Hash) -> Self {
        Self {
            staging_dir: TempDir::new().unwrap().into_persistent(),
            jcli,
            genesis_hash,
        }
    }

    fn staging_file(&self) -> ChildPath {
        self.staging_dir.child("transaction.tx")
    }

    pub fn staging_file_path(&self) -> PathBuf {
        PathBuf::from(self.staging_file().path())
    }

    pub fn build_transaction_from_utxo(
        self,
        utxo: &UTxOInfo,
        input_amount: Value,
        sender: &Wallet,
        output_amount: Value,
        receiver: &Wallet,
        valid_until: BlockDate,
    ) -> String {
        TransactionBuilder::new(self.jcli, self.genesis_hash)
            .new_transaction()
            .add_input(
                utxo.transaction_id(),
                utxo.index_in_transaction(),
                &input_amount.to_string(),
            )
            .add_output(&receiver.address().to_string(), output_amount)
            .set_expiry_date(valid_until)
            .finalize()
            .seal_with_witness_for_address(sender)
            .to_message()
    }

    pub fn new_transaction(&mut self) -> &mut Self {
        self.jcli
            .transaction()
            .new_transaction(self.staging_file().path());
        self
    }

    pub fn add_input(&mut self, tx_id: &Hash, tx_index: u8, amount: &str) -> &mut Self {
        self.jcli
            .transaction()
            .add_input(tx_id, tx_index, amount, self.staging_file().path());
        self
    }

    pub fn add_input_expect_fail(
        &mut self,
        tx_id: &Hash,
        tx_index: u8,
        amount: &str,
        expected_part: &str,
    ) -> &mut Self {
        self.jcli.transaction().add_input_expect_fail(
            tx_id,
            tx_index,
            amount,
            self.staging_file().path(),
            expected_part,
        );
        self
    }

    pub fn add_input_from_utxo_with_value(&mut self, utxo: &UTxOInfo, amount: Value) -> &mut Self {
        self.add_input(
            utxo.transaction_id(),
            utxo.index_in_transaction(),
            &amount.to_string(),
        );
        self
    }

    pub fn add_input_from_utxo(&mut self, utxo: &UTxOInfo) -> &mut Self {
        self.add_input(
            utxo.transaction_id(),
            utxo.index_in_transaction(),
            &utxo.associated_fund().to_string(),
        );
        self
    }

    pub fn add_certificate(&mut self, certificate: &str) -> &mut Self {
        self.jcli
            .transaction()
            .add_certificate(certificate, self.staging_file().path());
        self
    }

    pub fn add_account(&mut self, account_addr: &str, amount: &Value) -> &mut Self {
        self.jcli.transaction().add_account(
            account_addr,
            &amount.to_string(),
            self.staging_file().path(),
        );
        self
    }

    pub fn add_account_expect_fail(
        &mut self,
        account_addr: &str,
        amount: &str,
        expected_msg: &str,
    ) -> &mut Self {
        self.jcli.transaction().add_account_expect_fail(
            account_addr,
            amount,
            self.staging_file().path(),
            expected_msg,
        );
        self
    }

    pub fn add_account_from_legacy(&mut self, fund: &LegacyUTxO) -> &mut Self {
        self.add_account(&fund.address.to_string(), &fund.value)
    }

    pub fn add_output(&mut self, addr: &str, amount: Value) -> &mut Self {
        self.jcli
            .transaction()
            .add_output(addr, amount, self.staging_file().path());
        self
    }

    pub fn set_expiry_date(&mut self, valid_until: BlockDate) -> &mut Self {
        self.jcli
            .transaction()
            .set_expiry_date(valid_until, self.staging_file().path());
        self
    }

    pub fn finalize(&mut self) -> &mut Self {
        self.jcli.transaction().finalize(self.staging_file().path());
        self
    }

    pub fn finalize_with_fee(&mut self, address: &str, linear_fee: &LinearFee) -> &mut Self {
        self.jcli
            .transaction()
            .finalize_with_fee(address, linear_fee, self.staging_file().path());
        self
    }

    pub fn finalize_expect_fail(&self, expected_part: &str) {
        self.jcli
            .transaction()
            .finalize_expect_fail(self.staging_file().path(), expected_part);
    }

    pub fn add_auth<P: AsRef<Path>>(&mut self, key: P) -> &mut Self {
        self.jcli
            .transaction()
            .auth(key, self.staging_file().path());
        self
    }

    pub fn make_and_add_witness_default(&mut self, wallet: &Wallet) -> &mut Self {
        let witness = self.create_witness_from_wallet(wallet);
        self.make_witness(&witness);
        self.add_witness(&witness);
        self
    }

    pub fn seal_with_witness_for_address(&mut self, wallet: &Wallet) -> &mut Self {
        let witness = self.create_witness_from_wallet(wallet);
        self.seal_with_witness(&witness);
        self
    }

    pub fn seal_with_witness_default(
        &mut self,
        private_key: &str,
        transaction_type: &str,
        spending_key: Option<u32>,
    ) -> &mut Self {
        let witness = self.create_witness_from_key(private_key, transaction_type, spending_key);
        self.seal_with_witness(&witness);
        self
    }

    pub fn seal_with_witness(&mut self, witness: &Witness) -> &mut Self {
        self.make_witness(witness);
        self.add_witness(witness);
        self.seal();
        self
    }

    pub fn make_witness(&mut self, witness: &Witness) -> &mut Self {
        self.jcli.transaction().make_witness(witness);
        self
    }

    pub fn make_witness_expect_fail(&mut self, witness: &Witness, expected_msg: &str) -> &mut Self {
        self.jcli
            .transaction()
            .make_witness_expect_fail(witness, expected_msg);
        self
    }

    pub fn create_witness_from_wallet(&self, wallet: &Wallet) -> Witness {
        match wallet {
            Wallet::Account(account) => self.create_witness_from_key(
                &account.signing_key().to_bech32_str(),
                "account",
                Some(account.internal_counter().into()),
            ),
            Wallet::UTxO(utxo) => {
                self.create_witness_from_key(&utxo.last_signing_key().to_bech32_str(), "utxo", None)
            }
            Wallet::Delegation(delegation) => self.create_witness_from_key(
                &delegation.last_signing_key().to_bech32_str(),
                "utxo",
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
        let transaction_id = self.transaction_id();
        Witness::new(
            &self.staging_dir,
            &self.genesis_hash,
            &transaction_id,
            addr_type,
            private_key,
            spending_key,
        )
    }

    pub fn create_witness_default(&self, addr_type: &str, spending_key: Option<u32>) -> Witness {
        let private_key = self.jcli.key().generate_default();
        self.create_witness_from_key(&private_key, addr_type, spending_key)
    }

    pub fn add_witness_expect_fail(&mut self, witness: &Witness, expected_part: &str) {
        self.jcli.transaction().add_witness_expect_fail(
            witness,
            self.staging_file().path(),
            expected_part,
        );
    }

    pub fn add_witness(&mut self, witness: &Witness) -> &mut Self {
        self.jcli
            .transaction()
            .add_witness(witness, self.staging_file().path());
        self
    }

    pub fn seal(&mut self) -> &mut Self {
        self.jcli.transaction().seal(self.staging_file().path());
        self
    }

    pub fn to_message(&self) -> String {
        self.jcli
            .transaction()
            .convert_to_message(self.staging_file().path())
    }

    pub fn to_message_expect_fail(&self, expected_msg: &str) {
        self.jcli
            .transaction()
            .convert_to_message_expect_fail(self.staging_file().path(), expected_msg);
    }

    pub fn transaction_id(&self) -> Hash {
        self.jcli.transaction().id(self.staging_file().path())
    }

    pub fn info(&self, format: &str) -> String {
        self.jcli
            .transaction()
            .info(format, self.staging_file().path())
    }

    pub fn fragment_id(&self) -> Hash {
        let fragment_hex = self.to_message();
        let fragment_bytes = hex::decode(&fragment_hex).expect("Failed to parse message hex");
        Fragment::deserialize(fragment_bytes.as_slice())
            .expect("Failed to parse message")
            .hash()
            .into()
    }
}
