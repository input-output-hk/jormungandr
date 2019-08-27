#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;

use crate::common::configuration;

#[derive(Debug)]
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
            .arg(&stake_pool_id)
            .arg(&delegation_id);
        command
    }

    pub fn get_stake_key_registration_command(&self, delegation_key: &str) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("certificate")
            .arg("new")
            .arg("stake-key-registration")
            .arg(&delegation_key);
        command
    }

    pub fn get_stake_pool_registration_command(
        &self,
        kes_key: &str,
        serial_id: &str,
        vrf_key: &str,
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
            .arg("--serial")
            .arg(&serial_id)
            // The following are hardcoded, but will need testing
            .arg("--start-validity")
            .arg("0")
            .arg("--management-threshold")
            .arg("1")
            .arg("--owner")
            .arg("ed25519_pk12tx2erdy6m3xntfsgf8t2cyscjv0ls73974ma6a4rwfs3v2aup9q3qys5m");
        command
    }

    pub fn get_stake_pool_id_command(
        &self,
        input_file: &PathBuf,
        output_file: &PathBuf,
    ) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("certificate")
            .arg("get-stake-pool-id")
            .arg(&input_file)
            .arg(&output_file);
        command
    }

    pub fn get_sign_command(
        &self,
        signing_key: &PathBuf,
        input_file: &PathBuf,
        output_file: &PathBuf,
    ) -> Command {
        let mut command = Command::new(configuration::get_jcli_app().as_os_str());
        command
            .arg("certificate")
            .arg("sign")
            .arg(&signing_key.as_os_str())
            .arg(&input_file.as_os_str())
            .arg(&output_file.as_os_str());
        command
    }
}
