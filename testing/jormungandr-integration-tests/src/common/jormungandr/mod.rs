mod configuration_builder;
pub mod process;
pub mod starter;
use chain_core::property::Fragment as _;
use chain_impl_mockchain::fragment::Fragment;
use chain_impl_mockchain::fragment::FragmentId;
pub use configuration_builder::ConfigurationBuilder;
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::BlockDate;
use jormungandr_lib::interfaces::FragmentLog;
use jormungandr_testing_utils::testing::MemPoolCheck;
pub use process::*;
pub use starter::*;
use std::collections::HashMap;
use thiserror::Error;

use jormungandr_testing_utils::testing::{FragmentNode, FragmentNodeError};

#[derive(Error, Debug)]
pub enum JormungandrError {
    #[error("error in logs. Error lines: {error_lines}, full content:{logs}")]
    ErrorInLogs { logs: String, error_lines: String },
    #[error("error(s) in log detected: port already in use")]
    PortAlreadyInUse,
}

impl FragmentNode for JormungandrProcess {
    fn alias(&self) -> &str {
        self.alias()
    }
    fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, FragmentNodeError> {
        //TODO: implement conversion
        self.rest()
            .fragment_logs()
            .map_err(|e| FragmentNodeError::ListFragmentError(e.to_string()))
    }
    fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, FragmentNodeError> {
        self.rest().send_fragment(fragment.clone()).map_err(|e| {
            FragmentNodeError::CannotSendFragment {
                reason: e.to_string(),
                alias: self.alias().to_string(),
                fragment_id: fragment.id(),
                logs: self.log_content(),
            }
        })
    }

    fn send_batch_fragments(
        &self,
        fragments: Vec<Fragment>,
    ) -> Result<Vec<MemPoolCheck>, FragmentNodeError> {
        self.rest()
            .send_fragment_batch(fragments.clone())
            .map_err(|e| FragmentNodeError::CannotSendFragmentBatch {
                reason: e.to_string(),
                alias: self.alias().to_string(),
                fragment_ids: fragments.iter().map(|x| x.id()).collect(),
                logs: FragmentNode::log_content(self),
            })
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
    fn log_content(&self) -> Vec<String> {
        self.logger.get_log_content()
    }
}
