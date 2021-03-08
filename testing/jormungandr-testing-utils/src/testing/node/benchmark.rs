use crate::testing::{Speed, SpeedBenchmarkDef, SpeedBenchmarkFinish, Timestamp};
use std::time::Duration;

use crate::testing::node::JormungandrLogger;

pub fn storage_loading_benchmark_from_log(
    log: &JormungandrLogger,
    name: &str,
    timeout: Duration,
) -> SpeedBenchmarkFinish {
    speed_benchmark_from_log(
        log,
        name,
        timeout,
        "storing blockchain",
        "Loaded from storage",
    )
}

pub fn speed_benchmark_from_log(
    log: &JormungandrLogger,
    name: &str,
    timeout: Duration,
    start_measurement: &str,
    stop_measurement: &str,
) -> SpeedBenchmarkFinish {
    let start_entry: Timestamp = log
        .get_lines()
        .into_iter()
        .find(|x| x.fields.msg.contains(start_measurement))
        .expect("cannot find start mesurement entry in log")
        .into();

    let stop_entry: Timestamp = log
        .get_lines()
        .into_iter()
        .find(|x| x.fields.msg.contains(stop_measurement))
        .expect("cannot find stop mesurement entry in log")
        .into();

    let definition = SpeedBenchmarkDef::new(name.to_string())
        .target(timeout)
        .clone();
    let speed = Speed::new(&start_entry, &stop_entry);

    SpeedBenchmarkFinish::new(definition, speed)
}
