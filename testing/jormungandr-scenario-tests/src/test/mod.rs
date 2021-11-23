#[cfg(test)]
pub mod comm;
#[cfg(test)]
pub mod features;
#[cfg(test)]
pub mod legacy;
#[cfg(test)]
pub mod network;
#[cfg(test)]
pub mod non_functional;
pub mod utils;

use jormungandr_testing_utils::testing::jormungandr::JormungandrProcess;
use jortestkit::prelude::*;

pub fn start_resources_monitor<S: Into<String>>(
    info: S,
    nodes: Vec<&JormungandrProcess>,
) -> ConsumptionBenchmarkRun {
    benchmark_consumption(info.into())
        .for_processes(nodes.iter().map(|x| x.as_named_process()).collect())
        .bare_metal_stake_pool_consumption_target()
        .start()
}
