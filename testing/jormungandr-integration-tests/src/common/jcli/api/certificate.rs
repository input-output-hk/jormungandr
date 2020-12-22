use crate::common::jcli::command::CertificateCommand;
use assert_cmd::assert::OutputAssertExt;
use assert_fs::{prelude::*, NamedTempFile};
use chain_impl_mockchain::vote::Choice;
use jormungandr_lib::interfaces::TaxType;
use jortestkit::file;
use jortestkit::process::output_extensions::ProcessOutput;
use std::path::Path;

#[derive(Debug)]
pub struct Certificate {
    command: CertificateCommand,
}

impl Certificate {
    pub fn new(command: CertificateCommand) -> Self {
        Self { command }
    }

    pub fn new_vote_plan<P: AsRef<Path>>(self, proposal_file: P) -> String {
        self.command
            .vote(proposal_file)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn new_public_vote_tally<S: Into<String>>(self, vote_plan_id: S) -> String {
        self.command
            .public_vote_tally(vote_plan_id)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn new_private_vote_tally<S: Into<String>, P: AsRef<Path>>(
        self,
        vote_plan_id: S,
        shares: P,
    ) -> String {
        self.command
            .private_vote_tally(vote_plan_id, shares)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn new_encrypted_vote_tally<S: Into<String>, P: AsRef<Path>>(
        self,
        vote_plan_id: S,
        shares: P,
    ) -> String {
        self.command
            .encrypted_vote_tally(vote_plan_id, shares)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn new_public_vote_cast<S: Into<String>>(
        self,
        vote_plan_id: S,
        proposal_idx: usize,
        choice: Choice,
    ) -> String {
        self.command
            .public_vote_cast(vote_plan_id.into(), proposal_idx, choice.as_byte())
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn new_private_vote_cast<S: Into<String>, P: Into<String>>(
        self,
        vote_plan_id: S,
        proposal_idx: usize,
        choice: Choice,
        option_size: usize,
        encrypting_key: P,
    ) -> String {
        let key_path = NamedTempFile::new("key_path").unwrap();
        key_path.write_str(&encrypting_key.into()).unwrap();

        self.command
            .private_vote_cast(
                choice.as_byte(),
                option_size,
                proposal_idx,
                vote_plan_id.into(),
                key_path.path(),
            )
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn new_stake_delegation<S: Into<String>, P: Into<String>>(
        self,
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
        self,
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

    pub fn stake_pool_id<P: AsRef<Path>>(self, input_file: P) -> String {
        println!("Running get stake pool id...");
        let temp_file = NamedTempFile::new("stake_pool.id").unwrap();
        self.command
            .stake_pool_id(input_file, temp_file.path())
            .build()
            .assert()
            .success();
        temp_file.assert(jortestkit::prelude::file_exists_and_not_empty());
        file::read_file(temp_file.path())
    }

    pub fn vote_plan_id<S: Into<String>>(self, cert: S) -> String {
        println!("Running get stake pool id...");
        let input_file = NamedTempFile::new("cert_file").unwrap();
        input_file.write_str(&cert.into()).unwrap();
        let temp_file = NamedTempFile::new("vote_plan.id").unwrap();
        self.command
            .vote_plan_id(input_file.path(), temp_file.path())
            .build()
            .assert()
            .success();
        temp_file.assert(jortestkit::prelude::file_exists_and_not_empty());
        file::read_file(temp_file.path())
    }

    pub fn sign<P: AsRef<Path>, Q: AsRef<Path>, R: AsRef<Path>>(
        self,
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

    pub fn new_stake_pool_retirement(self, stake_pool_id: &str) -> String {
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
