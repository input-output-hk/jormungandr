#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

use crate::common::configuration;

use jormungandr_lib::interfaces::TaxType;

#[derive(Default, Debug)]
pub struct CertificateCommands {}

impl CertificateCommands {
    pub fn new() -> CertificateCommands {
        CertificateCommands {}
    }

    pub fn get_new_stake_delegation_command(
        &self,
        stake_pool_id: &str,
        delegation_id: &str,
    ) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("certificate")
            .arg("new")
            .arg("stake-delegation")
            .arg(&delegation_id)
            .arg(&stake_pool_id);
        command
    }

    pub fn get_retire_command(&self, stake_pool_id: &str, retirement_time: u64) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("certificate")
            .arg("new")
            .arg("stake-pool-retirement")
            .arg("--pool-id")
            .arg(&stake_pool_id)
            .arg("--retirement-time")
            .arg(&retirement_time.to_string());
        command
    }

    pub fn get_vote_command(&self, proposal_file: &Path) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("certificate")
            .arg("new")
            .arg("vote-plan")
            .arg(proposal_file);
        command
    }

    pub fn get_stake_pool_registration_command(
        &self,
        kes_key: &str,
        vrf_key: &str,
        start_validity: u32,
        management_threshold: u32,
        owner_pk: &str,
        tax_type: Option<TaxType>,
    ) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("certificate")
            .arg("new")
            .arg("stake-pool-registration")
            .arg("--kes-key")
            .arg(&kes_key)
            .arg("--vrf-key")
            .arg(&vrf_key)
            .arg("--start-validity")
            .arg(&start_validity.to_string())
            .arg("--management-threshold")
            .arg(&management_threshold.to_string())
            .arg("--owner")
            .arg(&owner_pk);

        if let Some(tax_type) = tax_type {
            command
                .arg("--tax-fixed")
                .arg(tax_type.fixed.to_string())
                .arg("--tax-ratio")
                .arg(format!("{}", tax_type.ratio));

            if let Some(max_limit) = tax_type.max_limit {
                command.arg("--tax-limit").arg(max_limit.to_string());
            }
        }
        command
    }

    pub fn get_stake_pool_id_command(&self, input_file: &Path, output_file: &Path) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("certificate")
            .arg("get-stake-pool-id")
            .arg(input_file)
            .arg(output_file);
        command
    }

    pub fn get_sign_command(
        &self,
        signing_key: &Path,
        input_file: &Path,
        output_file: &Path,
    ) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("certificate")
            .arg("sign")
            .arg("--key")
            .arg(signing_key)
            .arg("--certificate")
            .arg(input_file)
            .arg("--output")
            .arg(output_file);
        command
    }
}
