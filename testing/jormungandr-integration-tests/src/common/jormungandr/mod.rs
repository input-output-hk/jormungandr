mod benchmark;
mod configuration_builder;
pub mod logger;
pub mod process;
pub mod rest;
pub mod starter;
pub use benchmark::storage_loading_benchmark_from_log;
use chain_impl_mockchain::fragment::Fragment;
use chain_impl_mockchain::fragment::FragmentId;
pub use configuration_builder::ConfigurationBuilder;
use jormungandr_lib::interfaces::FragmentLog;
use jormungandr_testing_utils::testing::MemPoolCheck;
pub use logger::{JormungandrLogger, LogEntry};
pub use process::*;
pub use rest::{JormungandrRest, RestError};
pub use starter::*;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use chain_core::property::Fragment as _;
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::BlockDate;

use jormungandr_testing_utils::testing::{FragmentNode, FragmentNodeError};

#[derive(Error, Debug)]
pub enum JormungandrError {
    #[error("error in logs. Error lines: {error_lines}, logs location: {log_location}, full content:{logs}")]
    ErrorInLogs {
        logs: String,
        log_location: PathBuf,
        error_lines: String,
    },
}

impl FragmentNode for JormungandrProcess {
    fn alias(&self) -> &str {
        self.alias()
    }
    fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, FragmentNodeError> {
        //TODO: implement conversion
        println!("{:?}", self.rest().fragment_logs());
        self.rest()
            .fragment_logs()
            .map_err(|e| FragmentNodeError::UnknownError)
    }
    fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, FragmentNodeError> {
        println!("Sending fragment: {}", fragment.id());
        let result = self.rest().send_fragment(fragment);
        println!("Result: {:?}", result);
        result.map_err(|_| FragmentNodeError::UnknownError)
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
        self.logger.get_log_content()
    }
}
