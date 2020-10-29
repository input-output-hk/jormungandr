use jormungandr_lib::interfaces::TaxType;
use jortestkit::process::output_extensions::ProcessOutput;

use crate::common::jcli::command::CertificateCommand;
use assert_cmd::assert::OutputAssertExt;
use assert_fs::prelude::*;
use assert_fs::{NamedTempFile, TempDir};
use jormungandr_testing_utils::testing::file;
use std::path::{Path, PathBuf};
#[derive(Debug)]
pub struct Certificate {
    command: CertificateCommand,
}

impl Certificate {
    pub fn new(command: CertificateCommand) -> Self {
        Self { command }
    }

    pub fn new_vote_plan<P: AsRef<Path>>(mut self, proposal_file: P) -> String {
        self.command
            .vote(proposal_file)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn new_stake_delegation<S: Into<String>, P: Into<String>>(
        mut self,
        stake_pool_id: S,
        delegation_id: P,
    ) -> String {
        println!("Running new stake delegation...");
        self.command
            .new_stake_delegation(stake_pool_id, delegation_id)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn new_stake_pool_registration(
        mut self,
        kes_key: &str,
        vrf_key: &str,
        start_validity: u32,
        management_threshold: u32,
        owner_pk: &str,
        tax_type: Option<TaxType>,
    ) -> String {
        println!("Running new stake pool registration...");
        self.command
            .stake_pool_registration(
                kes_key,
                vrf_key,
                start_validity,
                management_threshold,
                owner_pk,
                tax_type,
            )
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn stake_pool_id<P: AsRef<Path>>(mut self, input_file: P) -> String {
        println!("Running get stake pool id...");
        let temp_file = NamedTempFile::new("stake_pool.id").unwrap();
        self.command
            .stake_pool_id(input_file, temp_file.path())
            .build()
            .assert()
            .success();
        temp_file.assert(crate::predicate::file_exists_and_not_empty());
        file::read_file(temp_file.path())
    }

    pub fn sign<P: AsRef<Path>, Q: AsRef<Path>, R: AsRef<Path>>(
        mut self,
        signing_key: P,
        input_file: Q,
        output_file: R,
    ) -> String {
        println!("Running sign certification...");
        self.command
            .sign(&signing_key, &input_file, &output_file)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn new_stake_pool_retirement(mut self, stake_pool_id: &str) -> String {
        println!("Running create retirement certification...");
        self.command
            .retire(stake_pool_id, 0u64)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }
}
