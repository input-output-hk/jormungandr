use super::ScenarioProgressBar;
use crate::mjolnir_lib::{bootstrap::ClientLoadConfig, MjolnirError};
use assert_fs::TempDir;
use indicatif::{MultiProgress, ProgressBar};
use jormungandr_automation::{
    jormungandr::LogLevel,
    testing::{benchmark_speed, SpeedBenchmarkFinish},
};
use jormungandr_lib::interfaces::NodeState;
use std::{thread, time};
pub struct IterationBasedClientLoad {
    config: ClientLoadConfig,
    sync_iteration: u32,
}

impl IterationBasedClientLoad {
    pub fn new(config: ClientLoadConfig, sync_iteration: u32) -> Self {
        Self {
            config,
            sync_iteration,
        }
    }

    fn get_storage_name(&self, id: u32, iteration: u32) -> String {
        format!("storage_{}_{}", id, iteration)
    }

    fn start_node(
        &self,
        temp_dir: &TempDir,
        id: u32,
        iteration: u32,
        multi_progress: &MultiProgress,
    ) -> Result<thread::JoinHandle<Result<SpeedBenchmarkFinish, MjolnirError>>, MjolnirError> {
        let storage_folder_name = self.get_storage_name(id, iteration);

        let progress_bar = ScenarioProgressBar::new(
            multi_progress.add(ProgressBar::new(1)),
            &format!("[Node: {}, iter: {}]", id, iteration),
        );

        let node = super::start_node(&self.config, &storage_folder_name, temp_dir)?;
        let benchmark = benchmark_speed(&storage_folder_name).no_target().start();

        Ok(thread::spawn(move || loop {
            thread::sleep(time::Duration::from_secs(2));

            if let Some(last_loaded_block) = node.logger.last_validated_block_date() {
                progress_bar.set_progress(&format!("block: {}", last_loaded_block))
            }
            node.check_no_errors_in_log()?;
            progress_bar.set_error_lines(
                node.logger
                    .get_log_lines_with_level(LogLevel::ERROR)
                    .map(|x| x.to_string())
                    .collect(),
            );

            let stats = node.rest().stats()?;
            if stats.state == NodeState::Running {
                progress_bar.set_finished();
                return Ok(benchmark.stop());
            }
        }))
    }

    pub fn run(&self) -> Result<(), MjolnirError> {
        let m = MultiProgress::new();
        let mut results = vec![];

        for iter in 1..=self.sync_iteration {
            println!("Iteration {}", iter);

            let mut handles = vec![];
            let mut temp_dirs = vec![];

            for client_id in 1..=self.config.count() {
                let temp_dir = TempDir::new().unwrap();
                handles.push(self.start_node(&temp_dir, client_id, iter, &m)?);
                temp_dirs.push(temp_dir);
            }
            m.join_and_clear().unwrap();

            for handle in handles {
                results.push(
                    handle
                        .join()
                        .map_err(|_| MjolnirError::InternalClientError)?,
                );
            }
        }

        if self.config.measure() {
            for overall_result in results {
                if let Ok(measurement) = &overall_result {
                    println!("{}", measurement);
                }
            }
        }
        Ok(())
    }
}
