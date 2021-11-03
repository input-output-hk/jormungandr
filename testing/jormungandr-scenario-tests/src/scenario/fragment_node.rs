use crate::node::Node;
use chain_core::property::Fragment as _;
use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::{BlockDate, FragmentLog, FragmentsProcessingSummary};
use jormungandr_testing_utils::testing::network::NodeAlias;
use jormungandr_testing_utils::testing::{FragmentNode, FragmentNodeError, MemPoolCheck};
use std::collections::HashMap;

impl FragmentNode for Node {
    fn alias(&self) -> NodeAlias {
        self.alias()
    }
    fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, FragmentNodeError> {
        //TODO: implement conversion
        self.rest()
            .fragment_logs()
            .map_err(|_| FragmentNodeError::UnknownError)
    }
    fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, FragmentNodeError> {
        //TODO: implement conversion
        self.rest()
            .send_fragment(fragment)
            .map_err(|_| FragmentNodeError::UnknownError)
    }

    fn send_batch_fragments(
        &self,
        fragments: Vec<Fragment>,
        fail_fast: bool,
    ) -> std::result::Result<FragmentsProcessingSummary, FragmentNodeError> {
        self.rest()
            .send_fragment_batch(fragments.clone(), fail_fast)
            .map_err(|e| FragmentNodeError::CannotSendFragmentBatch {
                reason: e.to_string(),
                alias: self.alias(),
                fragment_ids: fragments.iter().map(|x| x.id()).collect(),
                logs: FragmentNode::log_content(self),
            })
    }

    fn log_pending_fragment(&self, fragment_id: FragmentId) {
        self.progress_bar()
            .log_info(format!("Fragment '{}' is still pending", fragment_id));
    }
    fn log_rejected_fragment(&self, fragment_id: FragmentId, reason: String) {
        self.progress_bar()
            .log_info(format!("Fragment '{}' rejected: {}", fragment_id, reason));
    }
    fn log_in_block_fragment(&self, fragment_id: FragmentId, date: BlockDate, block: Hash) {
        self.progress_bar().log_info(format!(
            "Fragment '{}' in block: {} ({})",
            fragment_id, block, date
        ));
    }
    fn log_content(&self) -> Vec<String> {
        self.logger().get_lines_as_string()
    }
}
