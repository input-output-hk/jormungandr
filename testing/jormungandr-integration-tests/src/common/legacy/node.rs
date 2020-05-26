use super::{BackwardCompatibleConfig, BackwardCompatibleRest};
use crate::{
    common::{
        explorer::Explorer,
        jcli_wrapper,
        jormungandr::{JormungandrError, JormungandrLogger},
    },
    mock::JormungandrClient,
};
use chain_impl_mockchain::{
    fee::LinearFee,
    fragment::{Fragment, FragmentId},
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{BlockDate, FragmentLog},
};
use jormungandr_testing_utils::testing::{FragmentNode, FragmentNodeError, MemPoolCheck, SyncNode};
use std::{collections::HashMap, path::PathBuf, process::Child, str::FromStr};
use yaml_rust::YamlLoader;

impl FragmentNode for BackwardCompatibleJormungandr {
    fn alias(&self) -> &str {
        self.alias()
    }
    fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, FragmentNodeError> {
        //TODO: implement conversion
        self.rest()
            .fragment_logs()
            .map_err(|_| FragmentNodeError::UnknownError)
    }
    fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, FragmentNodeError> {
        self.rest()
            .send_fragment(fragment)
            .map_err(|_| FragmentNodeError::UnknownError)
    }
    fn log_pending_fragment(&self, fragment_id: FragmentId) {
        println!("Fragment '{}' is still pending", fragment_id);
    }
    fn log_rejected_fragment(&self, fragment_id: FragmentId, reason: String) {
        println!("Fragment '{}' rejected: {}", fragment_id, reason);
    }
    fn log_in_block_fragment(&self, fragment_id: FragmentId, date: BlockDate, block: Hash) {
        println!("Fragment '{}' in block: {} ({})", fragment_id, block, date);
    }
    fn log_content(&self) -> String {
        self.logger().get_log_content()
    }
}

impl SyncNode for BackwardCompatibleJormungandr {
    fn alias(&self) -> &str {
        self.alias()
    }

    fn last_block_height(&self) -> u32 {
        let docs = YamlLoader::load_from_str(&self.rest().stats().unwrap()).unwrap();
        docs.get(0).unwrap()["lastBlockHeight"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap()
    }

    fn log_stats(&self) {
        println!("{:?}", self.rest().stats());
    }

    fn all_blocks_hashes(&self) -> Vec<chain_impl_mockchain::key::Hash> {
        todo!()
    }

    fn log_content(&self) -> String {
        self.logger().get_log_content()
    }

    fn get_lines_with_error_and_invalid(&self) -> Vec<String> {
        self.logger().get_lines_with_error_and_invalid().collect()
    }
    fn is_running(&self) -> bool {
        todo!()
    }
}

#[derive(Debug)]
pub struct BackwardCompatibleJormungandr {
    pub child: Child,
    pub logger: JormungandrLogger,
    pub config: BackwardCompatibleConfig,
    pub grpc_client: JormungandrClient,
    alias: String,
}

impl BackwardCompatibleJormungandr {
    pub fn from_config(child: Child, config: BackwardCompatibleConfig, alias: String) -> Self {
        Self::new(
            child,
            alias,
            config.log_file_path().expect("no log file defined"),
            config,
        )
    }

    pub fn alias(&self) -> &str {
        self.alias.as_str()
    }

    pub fn new(
        child: Child,
        alias: String,
        log_file_path: PathBuf,
        config: BackwardCompatibleConfig,
    ) -> Self {
        Self {
            child,
            alias,
            logger: JormungandrLogger::new(log_file_path.clone()),
            grpc_client: JormungandrClient::from_address(&config.get_node_address()).unwrap(),
            config,
        }
    }

    pub fn logger(&self) -> &JormungandrLogger {
        &self.logger
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

        if !error_lines.is_empty() {
            return Err(JormungandrError::ErrorInLogs {
                logs: self.logger.get_log_content(),
                log_location: self.logger.log_file_path.clone(),
                error_lines: format!("{:?}", error_lines),
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
