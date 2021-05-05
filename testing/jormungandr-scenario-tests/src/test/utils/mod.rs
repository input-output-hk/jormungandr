use crate::legacy::LegacyNodeController;
use crate::{
    node::{FragmentNode, NodeController},
    scenario::Controller,
    test::Result,
};
pub use jormungandr_testing_utils::testing::{SyncNode, SyncWaitParams};

use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{BlockDate, FragmentLog, NodeState},
};
use jormungandr_testing_utils::{
    testing::{Speed, Thresholds},
    wallet::Wallet,
};
use std::{collections::HashMap, time::Duration};

use chain_impl_mockchain::fragment::{Fragment, FragmentId};
pub use jormungandr_testing_utils::testing::{
    assert, assert_equals,
    node::LogLevel,
    sync::{
        measure_and_log_sync_time, measure_fragment_propagation_speed,
        measure_how_many_nodes_are_running,
    },
    FragmentNodeError, MeasurementReportInterval, MemPoolCheck,
};

pub fn wait(seconds: u64) {
    std::thread::sleep(Duration::from_secs(seconds));
}

pub fn measure_single_transaction_propagation_speed<A: SyncNode + FragmentNode + Send + Sized>(
    controller: &mut Controller,
    mut wallet1: &mut Wallet,
    wallet2: &Wallet,
    leaders: &[&A],
    sync_wait: Thresholds<Speed>,
    info: &str,
    report_node_stats_interval: MeasurementReportInterval,
) -> Result<()> {
    let node = leaders.iter().next().unwrap();
    let check = controller.fragment_sender().send_transaction(
        &mut wallet1,
        &wallet2,
        *node,
        1_000.into(),
    )?;
    let fragment_id = check.fragment_id();
    Ok(measure_fragment_propagation_speed(
        *fragment_id,
        leaders,
        sync_wait,
        info,
        report_node_stats_interval,
    )?)
}

impl SyncNode for NodeController {
    fn alias(&self) -> &str {
        self.alias()
    }

    fn last_block_height(&self) -> u32 {
        self.stats()
            .unwrap()
            .stats
            .unwrap()
            .last_block_height
            .unwrap()
            .parse()
            .unwrap()
    }

    fn log_stats(&self) {
        println!("Node: {} -> {:?}", self.alias(), self.stats());
    }

    fn tip(&self) -> Hash {
        self.tip().expect("cannot get tip from rest")
    }

    fn is_running(&self) -> bool {
        self.stats().unwrap().state == NodeState::Running
    }

    fn log_content(&self) -> String {
        self.logger().get_log_content()
    }

    fn get_lines_with_error_and_invalid(&self) -> Vec<String> {
        self.logger()
            .get_lines_with_level(LogLevel::ERROR)
            .map(|x| x.to_string())
            .collect()
    }
}

impl FragmentNode for LegacyNodeController {
    fn alias(&self) -> &str {
        self.alias()
    }
    fn fragment_logs(
        &self,
    ) -> std::result::Result<HashMap<FragmentId, FragmentLog>, FragmentNodeError> {
        //TODO: implement conversion
        self.fragment_logs()
            .map_err(|_| FragmentNodeError::UnknownError)
    }
    fn send_fragment(
        &self,
        fragment: Fragment,
    ) -> std::result::Result<MemPoolCheck, FragmentNodeError> {
        //TODO: implement conversion
        self.send_fragment(fragment)
            .map_err(|_| FragmentNodeError::UnknownError)
    }

    fn send_batch_fragments(
        &self,
        _fragments: Vec<Fragment>,
        _fail_fast: bool,
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

impl SyncNode for LegacyNodeController {
    fn alias(&self) -> &str {
        self.alias()
    }

    fn last_block_height(&self) -> u32 {
        self.stats().unwrap()["lastBlockHeight"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap()
    }

    fn log_stats(&self) {
        println!("Node: {} -> {:?}", self.alias(), self.stats());
    }

    fn tip(&self) -> Hash {
        self.tip().expect("cannot get tip from rest")
    }

    fn log_content(&self) -> String {
        self.logger().get_log_content()
    }

    fn get_lines_with_error_and_invalid(&self) -> Vec<String> {
        self.logger()
            .get_lines_with_level(LogLevel::ERROR)
            .map(|x| x.to_string())
            .collect()
    }

    fn is_running(&self) -> bool {
        self.stats().unwrap()["state"].as_str().unwrap() == "Running"
    }
}
