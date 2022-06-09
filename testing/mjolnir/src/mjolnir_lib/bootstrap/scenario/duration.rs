use super::ScenarioProgressBar;
use crate::mjolnir_lib::{bootstrap::ClientLoadConfig, MjolnirError};
use assert_fs::TempDir;
use indicatif::{MultiProgress, ProgressBar};
use jormungandr_automation::{
    jormungandr::{JormungandrProcess, LogLevel},
    testing::{benchmark_speed, SpeedBenchmarkFinish, SpeedBenchmarkRun},
};
use jormungandr_lib::interfaces::NodeState;
use std::{
    thread,
    time::{Duration, Instant},
};

pub struct DurationBasedClientLoad {
    config: ClientLoadConfig,
    duration: u64,
}

impl DurationBasedClientLoad {
    pub fn new(config: ClientLoadConfig, duration: u64) -> Self {
        Self { config, duration }
    }

    pub fn run(&self) -> Result<(), MjolnirError> {
        let m = MultiProgress::new();
        let mut results = vec![];
        let mut handles = vec![];

        for client_id in 1..=self.config.count() {
            handles.push(self.run_single(client_id, self.duration, &m)?);
        }
        m.join_and_clear().unwrap();

        for handle in handles {
            results.push(
                handle
                    .join()
                    .map_err(|_| MjolnirError::InternalClientError)?,
            );
        }

        if self.config.measure() {
            for overall_result in results {
                if let Ok(measurements) = &overall_result {
                    for measurement in measurements {
                        println!("{}", measurement);
                    }
                }
            }
        }
        Ok(())
    }

    fn wait_for_bootstrap_phase_completed(
        node: &JormungandrProcess,
        instant: &Instant,
        target: u64,
        benchmark: SpeedBenchmarkRun,
        progress_bar: &ScenarioProgressBar,
    ) -> Result<Option<SpeedBenchmarkFinish>, MjolnirError> {
        loop {
            thread::sleep(Duration::from_secs(2));
            progress_bar.set_progress(&format!(
                "progress: {}/{}",
                instant.elapsed().as_secs(),
                target
            ));

            node.check_no_errors_in_log()?;

            progress_bar.set_error_lines(
                node.logger
                    .get_log_lines_with_level(LogLevel::ERROR)
                    .map(|x| x.to_string())
                    .collect(),
            );

            if instant.elapsed().as_secs() > target {
                return Ok(None);
            }

            let stats = node.rest().stats()?;

            if stats.state == NodeState::Running {
                progress_bar.set_finished();
                return Ok(Some(benchmark.stop()));
            }
        }
    }

    fn run_single(
        &self,
        id: u32,
        duration: u64,
        multi_progress: &MultiProgress,
    ) -> Result<thread::JoinHandle<Result<Vec<SpeedBenchmarkFinish>, MjolnirError>>, MjolnirError>
    {
        let temp_dir = TempDir::new().unwrap().into_persistent();
        let storage_folder_name = format!("client_{}", id);

        let progress_bar = ScenarioProgressBar::new(
            multi_progress.add(ProgressBar::new(1)),
            &format!("[Node: {}]", id),
        );
        let mut benchmarks = vec![];
        let timer = Instant::now();

        let config = self.config.clone();
        let mut node = super::start_node(&config, &storage_folder_name, &temp_dir)?;

        Ok(thread::spawn(move || {
            loop {
                let benchmark = benchmark_speed(&storage_folder_name.clone())
                    .no_target()
                    .start();
                let benchmark_result = Self::wait_for_bootstrap_phase_completed(
                    &node,
                    &timer,
                    duration,
                    benchmark,
                    &progress_bar,
                )?;

                match benchmark_result {
                    Some(benchmark) => benchmarks.push(benchmark),
                    // if there are no new benchmarks it means that test finished
                    None => return Ok(benchmarks),
                };

                node.shutdown();
                thread::sleep(Duration::from_secs(config.pace()));
                node = super::start_node(&config, &storage_folder_name, &temp_dir)?;
            }
        }))
    }
}
