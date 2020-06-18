use super::commands::CertificateCommands;

use crate::common::{
    file_utils, process_assert,
    process_utils::{self, output_extensions::ProcessOutput},
};
use jormungandr_lib::interfaces::TaxType;

use assert_fs::prelude::*;
use assert_fs::{NamedTempFile, TempDir};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Default)]
pub struct JCLICertificateWrapper {
    commands: CertificateCommands,
}

impl JCLICertificateWrapper {
    pub fn new() -> JCLICertificateWrapper {
        JCLICertificateWrapper {
            commands: CertificateCommands::new(),
        }
    }

    pub fn assert_new_vote_plan(&self, proposal_file: &Path) -> String {
        self.assert_new_certificate(self.commands.get_vote_command(proposal_file))
    }

    pub fn assert_new_signed_vote_plan(
        &self,
        proposal_file: &Path,
        stake_key_file: &Path,
    ) -> PathBuf {
        let temp_dir = TempDir::new().unwrap();
        let cert = self.assert_new_vote_plan(proposal_file);

        let cert_file = temp_dir.child("vote_plan.cert");
        cert_file.write_str(&cert).unwrap();

        let signcert_file = temp_dir.child("vote_plan.signcert");
        self.assert_sign(&stake_key_file, cert_file.path(), signcert_file.path());
        PathBuf::from(signcert_file.path())
    }

    fn assert_new_certificate(&self, command: Command) -> String {
        let output = process_utils::run_process_and_get_output(command);
        let certification = output.as_single_line();
        process_assert::assert_process_exited_successfully(output);
        certification
    }

    pub fn assert_new_stake_delegation(&self, stake_pool_id: &str, delegation_id: &str) -> String {
        println!("Running new stake delegation...");
        self.assert_new_certificate(
            self.commands
                .get_new_stake_delegation_command(&stake_pool_id, &delegation_id),
        )
    }

    pub fn assert_new_stake_pool_registration(
        &self,
        kes_key: &str,
        vrf_key: &str,
        start_validity: u32,
        management_threshold: u32,
        owner_pk: &str,
        tax_type: Option<TaxType>,
    ) -> String {
        println!("Running new stake pool registration...");
        self.assert_new_certificate(self.commands.get_stake_pool_registration_command(
            &kes_key,
            &vrf_key,
            start_validity,
            management_threshold,
            owner_pk,
            tax_type,
        ))
    }

    pub fn assert_get_stake_pool_id(&self, input_file: &Path) -> String {
        println!("Running get stake pool id...");
        let temp_file = NamedTempFile::new("stake_pool.id").unwrap();
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_stake_pool_id_command(&input_file, temp_file.path()),
        );
        process_assert::assert_process_exited_successfully(output);
        temp_file.assert(crate::predicate::file_exists_and_not_empty());
        file_utils::read_file(temp_file.path())
    }

    pub fn assert_sign(&self, signing_key: &Path, input_file: &Path, output_file: &Path) -> String {
        println!("Running sign certification...");
        let output = process_utils::run_process_and_get_output(self.commands.get_sign_command(
            &signing_key,
            &input_file,
            &output_file,
        ));
        let certification = output.as_single_line();
        process_assert::assert_process_exited_successfully(output);
        certification
    }

    pub fn assert_new_signed_stake_pool_cert(
        &self,
        pool_kes_pk: &str,
        pool_vrf_pk: &str,
        stake_key_file: &Path,
        start_validity: u32,
        management_threshold: u32,
        owner_pk: &str,
        tax_type: Option<TaxType>,
    ) {
        let temp_dir = TempDir::new().unwrap();

        let stake_pool_cert = self.assert_new_stake_pool_registration(
            &pool_kes_pk,
            &pool_vrf_pk,
            start_validity,
            management_threshold,
            owner_pk,
            tax_type,
        );
        let stake_pool_cert_file = temp_dir.child("stake_pool.cert");
        stake_pool_cert_file.write_str(&stake_pool_cert).unwrap();

        let stake_pool_signcert_file = temp_dir.child("stake_pool.signcert");
        self.assert_sign(
            &stake_key_file,
            stake_pool_cert_file.path(),
            stake_pool_signcert_file.path(),
        );
    }

    pub fn assert_new_signed_stake_pool_delegation(
        &self,
        stake_pool_id: &str,
        stake_key_pub: &str,
        stake_key_file: &Path,
    ) -> String {
        let temp_dir = TempDir::new().unwrap();

        let stake_delegation_cert =
            self.assert_new_stake_delegation(&stake_pool_id, &stake_key_pub);

        let stake_delegation_cert_file = temp_dir.child("stake_delegation.cert");
        stake_delegation_cert_file
            .write_str(&stake_delegation_cert)
            .unwrap();
        let stake_delegation_signcert_file = temp_dir.child("stake_delegation.signcert");

        self.assert_sign(
            &stake_key_file,
            stake_delegation_cert_file.path(),
            stake_delegation_signcert_file.path(),
        );
        file_utils::read_file(stake_delegation_signcert_file.path())
    }

    pub fn assert_new_stake_pool_retirement(&self, stake_pool_id: &str) -> String {
        println!("Running create retirement certification...");
        let output = process_utils::run_process_and_get_output(
            self.commands.get_retire_command(&stake_pool_id, 0u64),
        );
        let certification = output.as_single_line();
        process_assert::assert_process_exited_successfully(output);
        certification
    }
}
