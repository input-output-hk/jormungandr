use crate::testing::{
    benchmark_speed,
    verify::{assert_equals, Error as VerificationError},
    Speed, Thresholds,
};

mod measure;
mod node;
mod report;
mod wait;

use jormungandr_lib::time::Duration as LibsDuration;
pub use measure::*;
pub use node::{SyncNode, SyncNodeError, SyncNodeRecord};
pub use report::{MeasurementReportInterval, MeasurementReporter};
pub use wait::SyncWaitParams;

pub fn ensure_node_is_in_sync_with_others(
    target_node: &(impl SyncNode + Send),
    other_nodes: Vec<&(impl SyncNode + Send)>,
    sync_wait: Thresholds<Speed>,
    info: &str,
) -> Result<(), SyncNodeError> {
    let benchmark = benchmark_speed(info.to_owned())
        .with_thresholds(sync_wait)
        .start();

    while !benchmark.timeout_exceeded() {
        let target_node_block_height = target_node.last_block_height();

        let max_block_height: u32 = other_nodes
            .iter()
            .map(|node| node.last_block_height())
            .max()
            .expect("unable to retrieve block height from sync nodes");

        if target_node_block_height >= max_block_height {
            benchmark.stop();
            return Ok(());
        }
    }

    let other_nodes_records: Vec<SyncNodeRecord> = other_nodes
        .iter()
        .map(|x| SyncNodeRecord::new(x.alias(), x.last_block_height()))
        .collect();

    let target_node = SyncNodeRecord::new(target_node.alias(), target_node.last_block_height());

    Err(SyncNodeError::TimeoutWhenSyncingTargetNode {
        target_node,
        sync_nodes: other_nodes_records,
        timeout: benchmark.definition().thresholds().unwrap().max().into(),
    })
}

pub fn ensure_nodes_are_in_sync<A: SyncNode + ?Sized>(
    sync_wait: SyncWaitParams,
    nodes: &[&A],
) -> Result<(), VerificationError> {
    if nodes.len() < 2 {
        return Ok(());
    }

    wait_for_nodes_sync(&sync_wait);
    let duration: LibsDuration = sync_wait.wait_time().into();
    let first_node = nodes.iter().next().unwrap();

    let expected_tip = first_node.tip();
    let block_height = first_node.last_block_height();

    for node in nodes.iter().skip(1) {
        let tip = node.tip();
        assert_equals(
            &expected_tip,
            &tip,
            &format!("nodes are out of sync (different block hashes) after sync grace period: ({}) . Left node: alias: {}, content: {}, Right node: alias: {}, content: {}",
                duration,
                first_node.alias(),
                first_node.log_content(),
                node.alias(),
                node.log_content()),
        )?;
        assert_equals(
            &block_height,
            &node.last_block_height(),
            &format!("nodes are out of sync (different block height) after sync grace period: ({}) . Left node: alias: {}, content: {}, Right node: alias: {}, content: {}",
                duration,
                first_node.alias(),
                first_node.log_content(),
                node.alias(),
                node.log_content()
                ),
        )?;
    }
    Ok(())
}

pub fn wait_for_nodes_sync(sync_wait_params: &SyncWaitParams) {
    let wait_time = sync_wait_params.wait_time();
    std::thread::sleep(wait_time);
}
