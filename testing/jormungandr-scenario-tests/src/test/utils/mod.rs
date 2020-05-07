mod wait;

use crate::legacy::LegacyNodeController;
use crate::{
    node::NodeController,
    scenario::Controller,
    test::{ErrorKind, Result},
};
use chain_impl_mockchain::key::Hash;
use jormungandr_lib::{
    interfaces::{FragmentStatus, NodeState},
    time::Duration as LibsDuration,
};
use jormungandr_testing_utils::testing::{
    benchmark_efficiency, benchmark_speed, network_builder::Wallet, Speed, Thresholds,
};
use std::{
    fmt, thread,
    time::{Duration, SystemTime},
};
pub use wait::SyncWaitParams;

pub fn wait_for_nodes_sync(sync_wait_params: &SyncWaitParams) {
    let wait_time = sync_wait_params.wait_time();
    std::thread::sleep(wait_time);
}

pub fn wait(seconds: u64) {
    std::thread::sleep(Duration::from_secs(seconds));
}

pub fn get_nodes_block_height_summary(nodes: Vec<&NodeController>) -> Vec<String> {
    nodes
        .iter()
        .map({
            |node| {
                return format!(
                    "node '{}' has block height: '{:?}'\n",
                    node.alias(),
                    node.stats().unwrap().stats.unwrap().last_block_height
                );
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub enum MeasurementReportInterval {
    Standard,
    Long,
}

impl Into<u32> for MeasurementReportInterval {
    fn into(self) -> u32 {
        match self {
            Self::Standard => 20,
            Self::Long => 100,
        }
    }
}

pub struct MeasurementReporter {
    interval: u32,
    counter: u32,
}

impl MeasurementReporter {
    pub fn new(interval: MeasurementReportInterval) -> Self {
        Self {
            interval: interval.into(),
            counter: 0u32,
        }
    }

    pub fn is_interval_reached(&self) -> bool {
        self.counter >= self.interval
    }

    pub fn do_if_interval_reached<F: std::marker::Send>(&self, method: F)
    where
        F: Fn(),
    {
        if self.is_interval_reached() {
            method();
        }
    }

    pub fn do_if_interval_reached_and_inc<F: std::marker::Send>(&mut self, method: F)
    where
        F: Fn(),
    {
        if self.is_interval_reached() {
            method();
            self.counter = 0;
        } else {
            self.increment();
        }
    }

    pub fn increment(&mut self) {
        self.counter = self.counter + 1;
    }
}

pub fn measure_single_transaction_propagation_speed(
    controller: &mut Controller,
    mut wallet1: &mut Wallet,
    wallet2: &Wallet,
    leaders: Vec<&NodeController>,
    sync_wait: Thresholds<Speed>,
    info: &str,
    report_node_stats_interval: MeasurementReportInterval,
) -> Result<()> {
    let node = leaders.iter().next().unwrap();
    let check = controller.wallet_send_to(&mut wallet1, &wallet2, &node, 1_000.into())?;
    let fragment_id = check.fragment_id().clone();
    let benchmark = benchmark_speed(info.to_owned())
        .with_thresholds(sync_wait)
        .start();

    let leaders_nodes_count = leaders.len() as u32;
    let mut report_node_stats = MeasurementReporter::new(report_node_stats_interval);
    let mut leaders_ids: Vec<u32> = (1..=leaders_nodes_count).collect();

    while !benchmark.timeout_exceeded() {
        leaders_ids.retain(|leader_id| {
            let leader_index_usize = (leader_id - 1) as usize;
            let leader: &NodeController = leaders.get(leader_index_usize).unwrap();
            let fragment_logs = leader.fragment_logs().unwrap();
            report_node_stats.do_if_interval_reached(|| {
                println!("Node: {} -> {:?}", leader.alias(), leader.fragment_logs())
            });

            !fragment_logs.iter().any(|(id, _)| *id == fragment_id)
        });
        report_node_stats.increment();

        if leaders_ids.is_empty() {
            benchmark.stop().print();
            break;
        }
    }
    print_error_for_failed_leaders(leaders_ids, leaders);
    Ok(())
}

pub trait SyncNode {
    fn alias(&self) -> &str;
    fn last_block_height(&self) -> u32;
    fn log_stats(&self);
    fn all_blocks_hashes(&self) -> Vec<Hash>;
    fn log_content(&self) -> String;
}

//.stats()?.stats.unwrap().
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

    fn all_blocks_hashes(&self) -> Vec<Hash> {
        self.all_blocks_hashes().unwrap()
    }

    fn log_content(&self) -> String {
        self.logger().get_log_content()
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

    fn all_blocks_hashes(&self) -> Vec<Hash> {
        self.all_blocks_hashes().unwrap()
    }

    fn log_content(&self) -> String {
        self.logger().get_log_content()
    }
}

pub fn measure_and_log_sync_time<A: SyncNode + ?Sized>(
    nodes: Vec<&A>,
    sync_wait: Thresholds<Speed>,
    info: &str,
    report_node_stats_interval: MeasurementReportInterval,
) -> Result<()> {
    let benchmark = benchmark_speed(info.to_owned())
        .with_thresholds(sync_wait)
        .start();

    let mut report_node_stats_counter = 0u32;
    let interval: u32 = report_node_stats_interval.into();

    while !benchmark.timeout_exceeded() {
        let block_heights: Vec<u32> = nodes
            .iter()
            .map(|node| {
                if report_node_stats_counter >= interval {
                    node.log_stats();
                }
                node.last_block_height()
            })
            .collect();

        if report_node_stats_counter >= interval {
            println!(
                "Measuring sync time... current block heights: {:?}",
                block_heights
            );
            report_node_stats_counter = 0;
        } else {
            report_node_stats_counter = report_node_stats_counter + 1;
        }

        let max_block_height = block_heights.iter().cloned().max().unwrap();
        if block_heights
            .iter()
            .cloned()
            .filter(|x| *x != max_block_height)
            .count()
            == 0
        {
            benchmark.stop().print();
            return Ok(());
        }
    }

    // we know it fails, this method is used only for reporting
    let result = assert_are_in_sync(SyncWaitParams::ZeroWait, nodes);
    benchmark.stop().print();
    result
}

pub fn assert_equals<A: fmt::Debug + PartialEq>(left: &A, right: &A, info: &str) -> Result<()> {
    if left != right {
        bail!(ErrorKind::AssertionFailed(format!(
            "{}. {:?} vs {:?}",
            info, left, right
        )))
    }
    Ok(())
}

pub fn assert_is_up(node: &NodeController) -> Result<()> {
    if !node.is_up() {
        bail!(ErrorKind::AssertionFailed(format!(
            "Node '{}' is not up. Logs: {}",
            node.alias(),
            node.log_content()
        )))
    }
    Ok(())
}

pub fn assert(statement: bool, info: &str) -> Result<()> {
    if !statement {
        bail!(ErrorKind::AssertionFailed(info.to_string()))
    }
    Ok(())
}

pub fn assert_is_in_block(status: FragmentStatus, node: &NodeController) -> Result<()> {
    if !status.is_in_a_block() {
        bail!(ErrorKind::AssertionFailed(format!(
            "fragment status sent to node: {} is not in block :({:?}). logs: {}",
            node.alias(),
            status,
            node.log_content()
        )))
    }
    Ok(())
}

pub fn assert_are_in_sync<A: SyncNode + ?Sized>(
    sync_wait: SyncWaitParams,
    nodes: Vec<&A>,
) -> Result<()> {
    if nodes.len() < 2 {
        return Ok(());
    }

    wait_for_nodes_sync(&sync_wait);
    let duration: LibsDuration = sync_wait.wait_time().into();
    let first_node = nodes.iter().next().unwrap();

    let expected_block_hashes = first_node.all_blocks_hashes();
    let block_height = first_node.last_block_height();

    for node in nodes.iter().skip(1) {
        let all_block_hashes = node.all_blocks_hashes();
        assert_equals(
            &expected_block_hashes,
            &all_block_hashes,
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

pub fn keep_sending_transaction_dispite_error(
    n: u32,
    controller: &mut Controller,
    mut wallet1: &mut Wallet,
    wallet2: &Wallet,
    node: &NodeController,
) -> Result<()> {
    for _ in 0..n {
        let check = controller.wallet_send_to(&mut wallet1, &wallet2, &node, 1_000.into());
        if let Err(err) = check {
            println!("ignoring error : {:?}", err);
        }
        thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}

pub fn sending_transactions_to_node_sequentially(
    n: u32,
    controller: &mut Controller,
    mut wallet1: &mut Wallet,
    mut wallet2: &mut Wallet,
    node: &NodeController,
) -> Result<()> {
    for _ in 0..n {
        let check = controller.wallet_send_to(&mut wallet1, &wallet2, &node, 1_000.into())?;
        let status = node.wait_fragment(Duration::from_secs(2), check)?;
        assert_is_in_block(status, &node)?;
        let check = controller.wallet_send_to(&mut wallet2, &wallet1, &node, 1_000.into())?;
        let status = node.wait_fragment(Duration::from_secs(2), check)?;
        assert_is_in_block(status, &node)?;
    }
    Ok(())
}

pub fn measure_how_many_nodes_are_running(leaders: Vec<&NodeController>, name: &str) {
    let leaders_nodes_count = leaders.len() as u32;

    let mut efficiency_benchmark_run = benchmark_efficiency(name)
        .target(leaders_nodes_count)
        .start();
    let mut leaders_ids: Vec<u32> = (1..=leaders_nodes_count).collect();
    let now = SystemTime::now();

    loop {
        if now.elapsed().unwrap().as_secs() > (10 * 60) {
            break;
        }
        std::thread::sleep(Duration::from_secs(10));

        leaders_ids.retain(|leader_id| {
            let leader_index_usize = (leader_id - 1) as usize;
            let leader: &NodeController = leaders.get(leader_index_usize).unwrap();
            if let Ok(stats) = leader.stats() {
                if let NodeState::Running = stats.state {
                    efficiency_benchmark_run.increment();
                    return false;
                }
            }
            return true;
        });

        if leaders_ids.is_empty() {
            break;
        }
    }

    print_error_for_failed_leaders(leaders_ids, leaders);

    efficiency_benchmark_run.stop().print()
}

fn print_error_for_failed_leaders(leaders_ids: Vec<u32>, leaders: Vec<&NodeController>) {
    if leaders_ids.is_empty() {
        return;
    }

    println!("Nodes which failed to bootstrap: ");
    for leader_id in leaders_ids {
        let leader_index_usize = (leader_id - 1) as usize;
        let leader = leaders.get(leader_index_usize).unwrap();
        let error_lines: Vec<String> = leader.logger().get_lines_with_error_and_invalid().collect();
        println!("{} - Error Logs: {:?}", leader.alias(), error_lines);
    }
}
