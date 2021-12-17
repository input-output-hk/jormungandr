use hersir::controller::Controller;
use jormungandr_testing_utils::testing::FragmentNode;
use jormungandr_testing_utils::testing::FragmentSender;
pub use jormungandr_testing_utils::testing::{
    assert, assert_equals,
    node::LogLevel,
    sync::{
        measure_and_log_sync_time, measure_fragment_propagation_speed,
        measure_how_many_nodes_are_running,
    },
    FragmentNodeError, MeasurementReportInterval, MemPoolCheck,
};
pub use jormungandr_testing_utils::testing::{SyncNode, SyncWaitParams};
use jormungandr_testing_utils::{
    testing::{Speed, Thresholds},
    wallet::Wallet,
};
use std::time::Duration;

pub fn wait(seconds: u64) {
    std::thread::sleep(Duration::from_secs(seconds));
}

pub fn measure_single_transaction_propagation_speed<A: SyncNode + FragmentNode + Send + Sized>(
    controller: &mut Controller,
    wallet1: &mut Wallet,
    wallet2: &Wallet,
    leaders: &[&A],
    sync_wait: Thresholds<Speed>,
    info: &str,
    report_node_stats_interval: MeasurementReportInterval,
) {
    let node = leaders.iter().next().unwrap();
    let check = FragmentSender::from(&controller.settings().block0)
        .send_transaction(wallet1, wallet2, *node, 1_000.into())
        .unwrap();
    let fragment_id = check.fragment_id();
    measure_fragment_propagation_speed(
        *fragment_id,
        leaders,
        sync_wait,
        info,
        report_node_stats_interval,
    )
    .unwrap()
}
