use super::{logger::JormungandrLogger, JormungandrError, JormungandrRest};
use crate::common::{
    configuration::JormungandrConfig,
    explorer::Explorer,
    jcli_wrapper,
    jormungandr::starter::{Starter, StartupError},
};
use chain_impl_mockchain::fee::LinearFee;
use jormungandr_lib::{crypto::hash::Hash, interfaces::TrustedPeer};
use std::{path::PathBuf, process::Child, str::FromStr};

#[derive(Debug)]
pub struct JormungandrProcess {
    pub child: Child,
    pub logger: JormungandrLogger,
    pub config: JormungandrConfig,
    alias: String,
}

impl JormungandrProcess {
    pub fn from_config(child: Child, config: JormungandrConfig, alias: String) -> Self {
        JormungandrProcess::new(child, alias, config.log_file_path().clone(), config)
    }

    pub fn new(
        child: Child,
        alias: String,
        log_file_path: PathBuf,
        config: JormungandrConfig,
    ) -> Self {
        JormungandrProcess {
            child: child,
            alias: alias,
            logger: JormungandrLogger::new(log_file_path.clone()),
            config: config,
        }
    }

    pub fn alias(&self) -> String {
        self.alias.clone()
    }

    pub fn rest(&self) -> JormungandrRest {
        JormungandrRest::new(self.config.clone())
    }

    pub fn shutdown(&self) {
        jcli_wrapper::assert_rest_shutdown(&self.config.get_node_address());
    }

    pub fn address(&self) -> poldercast::Address {
        self.config.node_config().p2p.public_address.clone()
    }

    pub fn log_stats(&self) {
        println!("{:?}", self.rest().stats());
    }

    pub fn assert_no_errors_in_log_with_message(&self, message: &str) {
        let error_lines = self.logger.get_lines_with_error().collect::<Vec<String>>();

        assert_eq!(
            error_lines.len(),
            0,
            "{} there are some errors in log ({:?}): {:?}",
            message,
            self.logger.log_file_path,
            error_lines,
        );
    }

    pub fn assert_no_errors_in_log(&self) {
        let error_lines = self.logger.get_lines_with_error().collect::<Vec<String>>();

        assert_eq!(
            error_lines.len(),
            0,
            "there are some errors in log ({:?}): {:?}",
            self.logger.log_file_path,
            error_lines
        );
    }

    pub fn check_no_errors_in_log(&self) -> Result<(), JormungandrError> {
        let error_lines = self.logger.get_lines_with_error().collect::<Vec<String>>();

        if error_lines.len() != 0 {
            return Err(JormungandrError::ErrorInLogs {
                logs: self.logger.get_log_content(),
                log_location: self.logger.log_file_path.clone(),
                error_lines: format!("{:?}", error_lines).to_owned(),
            });
        }
        Ok(())
    }

    pub fn rest_address(&self) -> String {
        self.config.get_node_address()
    }

    pub fn fees(&self) -> LinearFee {
        self.config.fees()
    }

    pub fn genesis_block_hash(&self) -> Hash {
        Hash::from_str(&self.config.genesis_block_hash()).unwrap()
    }

    pub fn config(&self) -> JormungandrConfig {
        self.config.clone()
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn explorer(&self) -> Explorer {
        Explorer::new(self.config.node_config().rest.listen.to_string())
    }

    pub fn as_trusted_peer(&self) -> TrustedPeer {
        self.config.as_trusted_peer()
    }

    pub fn launch(&mut self) -> Result<Self, StartupError> {
        let mut starter = Starter::new();
        starter.config(self.config());
        if *self.config().genesis_block_hash() != "" {
            starter.from_genesis_hash();
        }
        starter.start()
    }
}

impl Drop for JormungandrProcess {
    fn drop(&mut self) {
        self.logger.print_error_and_invalid_logs();
        match self.child.kill() {
            Err(e) => println!("Could not kill {}: {}", self.alias, e),
            Ok(_) => println!("Successfully killed {}", self.alias),
        }
    }
}
