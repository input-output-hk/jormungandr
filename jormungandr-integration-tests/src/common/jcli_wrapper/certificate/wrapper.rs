use super::commands::CertificateCommands;

use crate::common::file_assert;
use crate::common::file_utils;
use crate::common::process_assert;
use crate::common::process_utils;
use crate::common::process_utils::output_extensions::ProcessOutput;
use std::path::PathBuf;

#[derive(Debug)]
pub struct JCLICertificateWrapper {
    commands: CertificateCommands,
}

impl JCLICertificateWrapper {
    pub fn new() -> JCLICertificateWrapper {
        JCLICertificateWrapper {
            commands: CertificateCommands::new(),
        }
    }

    pub fn assert_new_stake_delegation(&self, stake_pool_id: &str, delegation_id: &str) -> String {
        println!("Running new stake delegation...");
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_new_stake_delegation_command(&stake_pool_id, &delegation_id),
        );
        let certification = output.as_single_line();
        process_assert::assert_process_exited_successfully(output);
        certification
    }

    pub fn assert_new_stake_key_registration(&self, delegation_key: &str) -> String {
        println!("Running new stake key registration...");
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_stake_key_registration_command(&delegation_key),
        );
        let certification = output.as_single_line();
        process_assert::assert_process_exited_successfully(output);
        certification
    }

    pub fn assert_new_stake_pool_registration(
        &self,
        kes_key: &str,
        serial_id: &str,
        vrf_key: &str,
    ) -> String {
        println!("Running new stake pool registration...");
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_stake_pool_registration_command(&kes_key, &serial_id, &vrf_key),
        );
        let certification = output.as_single_line();
        process_assert::assert_process_exited_successfully(output);
        certification
    }

    pub fn assert_get_stake_pool_id(&self, input_file: &PathBuf) -> String {
        println!("Running get stake pool id...");
        let stake_pool_id_file = file_utils::get_path_in_temp("stake_pool.id");
        let output = process_utils::run_process_and_get_output(
            self.commands
                .get_stake_pool_id_command(&input_file, &stake_pool_id_file),
        );
        process_assert::assert_process_exited_successfully(output);
        file_assert::assert_file_exists_and_not_empty(&stake_pool_id_file);
        let certification = file_utils::read_file(&stake_pool_id_file);
        certification
    }

    pub fn assert_sign(
        &self,
        signing_key: &PathBuf,
        input_file: &PathBuf,
        output_file: &PathBuf,
    ) -> String {
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
        node_id: &str,
        pool_vrf_pk: &str,
        stake_key_file: &PathBuf,
    ) -> PathBuf {
        let stake_pool_cert =
            self.assert_new_stake_pool_registration(&pool_kes_pk, &node_id, &pool_vrf_pk);
        let stake_pool_cert_file =
            file_utils::create_file_in_temp("stake_pool.cert", &stake_pool_cert);

        let stake_pool_signcert_file = file_utils::get_path_in_temp("stake_pool.signcert");
        self.assert_sign(
            &stake_key_file,
            &stake_pool_cert_file,
            &stake_pool_signcert_file,
        );
        stake_pool_signcert_file
    }

    pub fn assert_new_signed_stake_pool_delegation(
        &self,
        stake_pool_id: &str,
        stake_key_pub: &str,
        stake_key_file: &PathBuf,
    ) -> String {
        let stake_delegation_cert =
            self.assert_new_stake_delegation(&stake_pool_id, &stake_key_pub);

        let stake_delegation_cert_file =
            file_utils::create_file_in_temp("stake_delegation.cert", &stake_delegation_cert);
        let stake_delegation_signcert_file =
            file_utils::get_path_in_temp("stake_delegation.signcert");

        self.assert_sign(
            &stake_key_file,
            &stake_delegation_cert_file,
            &stake_delegation_signcert_file,
        );
        file_utils::read_file(&stake_delegation_signcert_file)
    }
}
