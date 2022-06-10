use super::{
    ensure_nodes_are_in_sync, MeasurementReportInterval, MeasurementReporter, SyncNode,
    SyncWaitParams,
};
use crate::{
    jormungandr::FragmentNode,
    testing::{benchmark_efficiency, benchmark_speed, Speed, Thresholds, VerificationError},
};
use chain_impl_mockchain::fragment::FragmentId;
use std::time::{Duration, SystemTime};

pub fn measure_how_many_nodes_are_running<A: SyncNode + ?Sized>(leaders: &[&A], name: &str) {
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
            let leader: &A = leaders.get(leader_index_usize).unwrap();
            if leader.is_running() {
                efficiency_benchmark_run.increment();
                return false;
            }
            true
        });

        if leaders_ids.is_empty() {
            break;
        }
    }

    print_error_for_failed_leaders(leaders_ids, leaders);

    efficiency_benchmark_run.stop().print()
}

fn print_error_for_failed_leaders<A: SyncNode + ?Sized>(leaders_ids: Vec<u32>, leaders: &[&A]) {
    if leaders_ids.is_empty() {
        return;
    }

    println!("Nodes which failed to bootstrap: ");
    for leader_id in leaders_ids {
        let leader_index_usize = (leader_id - 1) as usize;
        let leader = leaders.get(leader_index_usize).unwrap();
        println!(
            "{} - Error Logs: {:?}",
            leader.alias(),
            leader.get_lines_with_error_and_invalid()
        );
    }
}

pub fn measure_fragment_propagation_speed<A: FragmentNode + Sized>(
    fragment_id: FragmentId,
    leaders: &[&A],
    sync_wait: Thresholds<Speed>,
    info: &str,
    report_node_stats_interval: MeasurementReportInterval,
) -> Result<(), VerificationError> {
    let benchmark = benchmark_speed(info.to_owned())
        .with_thresholds(sync_wait)
        .start();

    let leaders_nodes_count = leaders.len() as u32;
    let mut report_node_stats = MeasurementReporter::new(report_node_stats_interval);
    let mut leaders_ids: Vec<u32> = (1..=leaders_nodes_count).collect();

    while !benchmark.timeout_exceeded() {
        leaders_ids.retain(|leader_id| {
            let leader_index_usize = (leader_id - 1) as usize;
            let leader: &A = leaders.get(leader_index_usize).unwrap();
            let fragment_logs = leader.fragment_logs().unwrap();
            let alias = FragmentNode::alias(leader);
            report_node_stats
                .do_if_interval_reached(|| println!("Node: {} -> {:?}", alias, fragment_logs));

            !fragment_logs.iter().any(|(id, _)| *id == fragment_id)
        });
        report_node_stats.increment();

        if leaders_ids.is_empty() {
            benchmark.stop().print();
            break;
        }
    }
    Ok(())
}

pub fn measure_and_log_sync_time<A: SyncNode + ?Sized>(
    nodes: &[&A],
    sync_wait: Thresholds<Speed>,
    info: &str,
    report_node_stats_interval: MeasurementReportInterval,
) -> Result<(), VerificationError> {
    let benchmark = benchmark_speed(info.to_owned())
        .with_thresholds(sync_wait)
        .start();

    let mut report_node_stats_counter = 0u32;
    let interval: u32 = report_node_stats_interval.into();

    while !benchmark.timeout_exceeded() {
        let tips = nodes
            .iter()
            .map(|node| {
                if report_node_stats_counter >= interval {
                    node.log_stats();
                }
                node.tip()
            })
            .collect::<Vec<_>>();

        if report_node_stats_counter >= interval {
            println!("Measuring sync time... current block tips: {:?}", tips);
            report_node_stats_counter = 0;
        } else {
            report_node_stats_counter += 1;
        }

        let first = tips.first().cloned();
        let stop = first
            .map(|tip| tips.into_iter().all(|t| t == tip))
            .unwrap_or(true);

        if stop {
            benchmark.stop().print();
            return Ok(());
        }
    }

    // we know it fails, this method is used only for reporting
    let result = ensure_nodes_are_in_sync(SyncWaitParams::ZeroWait, nodes);
    benchmark.stop().print();
    result
}
