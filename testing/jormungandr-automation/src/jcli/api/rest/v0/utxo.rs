use crate::jcli::command::rest::v0::UtxOCommand;
use assert_cmd::assert::OutputAssertExt;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{UTxOInfo, UTxOOutputInfo},
};
use jortestkit::prelude::ProcessOutput;
use std::str::FromStr;
pub struct UtxO {
    utxo_command: UtxOCommand,
}

impl UtxO {
    pub fn new(utxo_command: UtxOCommand) -> Self {
        Self { utxo_command }
    }

    pub fn assert_contains<S: Into<String>>(self, utxo: &UTxOInfo, host: S) {
        assert_eq!(self.get_by_item(utxo, host), *utxo)
    }

    pub fn get_by_item<S: Into<String>>(self, utxo: &UTxOInfo, host: S) -> UTxOInfo {
        self.get(
            utxo.transaction_id().to_string(),
            utxo.index_in_transaction(),
            host.into(),
        )
    }

    pub fn get<S: Clone + Into<String>>(
        self,
        fragment_id: S,
        output_index: u8,
        host: S,
    ) -> UTxOInfo {
        let content = self
            .utxo_command
            .fragment_id(fragment_id.clone().into())
            .output_index(output_index)
            .get()
            .host(host.into())
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string();

        serde_yaml::from_str::<UTxOOutputInfo>(&content)
            .expect("JCLI returned malformed UTxO")
            .into_utxo_info(Hash::from_str(&fragment_id.into()).unwrap(), output_index)
    }

    pub fn expect_item_not_found<S: Into<String>>(self, utxo: &UTxOInfo, host: S) {
        self.expect_not_found(
            &utxo.transaction_id().to_string(),
            utxo.index_in_transaction(),
            &host.into(),
        )
    }

    pub fn expect_not_found<S: Into<String>>(self, fragment_id: S, output_index: u8, host: S) {
        self.utxo_command
            .fragment_id(fragment_id.into())
            .output_index(output_index)
            .get()
            .host(host.into())
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains("404 Not Found"));
    }
}
