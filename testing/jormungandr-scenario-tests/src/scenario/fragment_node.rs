use crate::node::NodeController;
use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::{BlockDate, FragmentLog};
use jormungandr_testing_utils::testing::{FragmentNode, FragmentNodeError, MemPoolCheck};
use std::collections::HashMap;

impl FragmentNode for NodeController {
    fn alias(&self) -> &str {
        self.alias()
    }
    fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, FragmentNodeError> {
        //TODO: implement conversion
        self.fragment_logs()
            .map_err(|_| FragmentNodeError::UnknownError)
    }
    fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, FragmentNodeError> {
        //TODO: implement conversion
        self.send_fragment(fragment)
            .map_err(|_| FragmentNodeError::UnknownError)
    }

    fn send_batch_fragments(
        &self,
        _fragments: Vec<Fragment>,
    ) -> std::result::Result<Vec<MemPoolCheck>, FragmentNodeError> {
        //TODO implement
        unimplemented!()
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
