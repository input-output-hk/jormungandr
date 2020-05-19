use super::{BackwardCompatibleConfig, BackwardCompatibleRest};
use crate::common::{
    explorer::Explorer,
    jcli_wrapper,
    jormungandr::{JormungandrError, JormungandrLogger},
};
use chain_impl_mockchain::fee::LinearFee;
use jormungandr_lib::crypto::hash::Hash;
use std::{path::PathBuf, process::Child, str::FromStr};

#[derive(Debug)]
pub struct BackwardCompatibleJormungandr {
    pub child: Child,
    pub logger: JormungandrLogger,
    pub config: BackwardCompatibleConfig,
    alias: String,
}

impl BackwardCompatibleJormungandr {
    pub fn from_config(child: Child, config: BackwardCompatibleConfig, alias: String) -> Self {
        Self::new(
            child,
            alias,
            config.log_file_path().expect("no log file defined").clone(),
            config,
        )
    }

    pub fn new(
        child: Child,
        alias: String,
        log_file_path: PathBuf,
        config: BackwardCompatibleConfig,
    ) -> Self {
        Self {
            child: child,
            alias: alias,
            logger: JormungandrLogger::new(log_file_path.clone()),
            config: config,
        }
    }

    pub fn alias(&self) -> String {
        self.alias.clone()
    }

    pub fn rest(&self) -> BackwardCompatibleRest {
        BackwardCompatibleRest::new(self.config.get_node_address())
    }

    pub fn shutdown(&self) {
        jcli_wrapper::assert_rest_shutdown(&self.config.get_node_address());
    }

    pub fn fees(&self) -> LinearFee {
        self.config.fees()
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

    pub fn genesis_block_hash(&self) -> Hash {
        Hash::from_str(&self.config.genesis_block_hash).unwrap()
    }

    pub fn config(&self) -> BackwardCompatibleConfig {
        self.config.clone()
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn explorer(&self) -> Explorer {
        Explorer::new(self.rest_address())
    }
}

impl Drop for BackwardCompatibleJormungandr {
    fn drop(&mut self) {
        self.logger.print_error_and_invalid_logs();
        match self.child.kill() {
            Err(e) => println!("Could not kill {}: {}", self.alias, e),
            Ok(_) => println!("Successfully killed {}", self.alias),
        }
    }
}
