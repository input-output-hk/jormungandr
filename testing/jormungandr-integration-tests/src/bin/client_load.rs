use assert_fs::prelude::*;
use assert_fs::TempDir;
use chain_impl_mockchain::key::Hash;
use jormungandr_integration_tests::{
    common::{
        file_utils,
        jormungandr::{
            ConfigurationBuilder, JormungandrError, JormungandrProcess, RestError, Starter,
            StartupError,
        },
    },
    mock::client::JormungandrClient,
};
use jormungandr_lib::interfaces::{NodeState, TrustedPeer};
use std::path::PathBuf;
pub fn main() -> Result<(), ClientLoadError> {
    ClientLoad::from_args().exec()
}
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use jormungandr_testing_utils::testing::{
    benchmark_speed, SpeedBenchmarkFinish, SpeedBenchmarkRun,
};
use std::{fs, result::Result, str::FromStr, thread, time::Instant};
use structopt::StructOpt;
use thiserror::Error;
use tokio::runtime::Runtime;

#[derive(Error, Debug)]
pub enum ClientLoadError {
    #[error("cannot spawn not with version '{0}', looks like it's incorrect one")]
    VersionNotFound(String),
    #[error("cannot find node with alias(0). Please run 'describe' command ")]
    NodeAliasNotFound(String),
    #[error("cannot query rest")]
    RestError(#[from] RestError),
    #[error("cannot bootstrap node")]
    StartupError(#[from] StartupError),
    #[error("jormungandr error")]
    JormungandrError(#[from] JormungandrError),
    #[error("node client error")]
    InternalClientError,
}

#[derive(StructOpt, Debug)]
pub struct ClientLoad {
    /// Prints nodes related data, like stats,fragments etc.
    #[structopt(short = "c", long = "count", default_value = "3")]
    pub count: u32,
    /// address in format:
    /// /ip4/54.193.75.55/tcp/3000
    #[structopt(short = "a", long = "address")]
    pub address: String,

    #[structopt(short = "i", long = "ip", default_value = "127.0.0.1")]
    pub ip: String,

    /// amount of delay [seconds] between sync attempts
    #[structopt(short = "p", long = "pace", default_value = "0")]
    pub pace: u32,

    #[structopt(short = "d", long = "storage")]
    pub initial_storage: Option<PathBuf>,

    /// amount of delay [seconds] between sync attempts
    #[structopt(short = "r", long = "duration")]
    pub duration: Option<u32>,

    /// amount of delay [seconds] between sync attempts
    #[structopt(short = "n", long = "iterations")]
    pub sync_iteration: Option<u32>,

    #[structopt(short = "m", long = "measure")]
    pub measure: bool,
}

impl ClientLoad {
    fn get_block0_hash(&self) -> Hash {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                let grpc_client = JormungandrClient::from_address(&self.address).unwrap();
                return grpc_client.get_genesis_block_hash().await;
            })
            .into()
    }

    fn build_config(&self) -> ClientLoadConfig {
        let block0_hash = self.get_block0_hash();
        ClientLoadConfig::new(
            block0_hash,
            self.measure,
            self.count,
            self.address.clone(),
            self.ip.clone(),
            self.pace,
            self.initial_storage.clone(),
        )
    }

    pub fn exec(&self) -> Result<(), ClientLoadError> {
        if let Some(duration) = self.duration {
            return DurationBasedClientLoad::new(self.build_config(), duration).run()?;
        }

        if let Some(sync_iteration) = self.sync_iteration {
            return IterationBasedClientLoad::new(self.build_config(), sync_iteration).run()?;
        }

        panic!("no duration nor iteration target chosen");
    }
}

pub struct ClientLoadConfig {
    block0_hash: Hash,
    measure: bool,
    count: u32,
    address: String,
    ip: String,
    pace: u32,
    initial_storage: Option<PathBuf>,
}

impl ClientLoadConfig {
    pub fn new(
        block0_hash: Hash,
        measure: bool,
        count: u32,
        address: String,
        ip: String,
        pace: u32,
        initial_storage: Option<PathBuf>,
    ) -> Self {
        Self {
            block0_hash,
            measure,
            count,
            address,
            ip,
            pace,
            initial_storage,
        }
    }

    pub fn trusted_peer(&self) -> TrustedPeer {
        TrustedPeer {
            address: self.address.parse().unwrap(),
        }
    }
}

pub struct DurationBasedClientLoad {
    config: ClientLoadConfig,
    duration: u32,
}

impl DurationBasedClientLoad {
    pub fn new(config: ClientLoadConfig, duration: u32) -> Self {
        Self { config, duration }
    }

    pub fn run(&self) -> Result<(), ClientLoadError> {
        let m = MultiProgress::new();
        let mut results = vec![];
        let mut handles = vec![];
        let mut temp_dirs = vec![];

        for client_id in 1..=self.config.count {
            let temp_dir = TempDir::new().unwrap();
            handles.push(self.start_node(&temp_dir, client_id, iter, &m)?);
            temp_dirs.push(temp_dir);
        }
        m.join_and_clear().unwrap();

        for handle in handles {
            results.push(
                handle
                    .join()
                    .map_err(|_| ClientLoadError::InternalClientError)?,
            );
        }

        if self.config.measure {
            for result in results {
                if let Ok(result) = &result {
                    println!("{}", result);
                }
            }
        }
        Ok(())
    }

    fn wait_for_bootstrap_phase_completed(
        &self,
        node: JormungandrProcess,
        benchmark: SpeedBenchmarkRun,
        progress_bar: ProgressBar,
    ) -> Result<Option<SpeedBenchmarkFinish>, ClientLoadError> {
        loop {
            let stats = node.rest().stats()?;
            let log_entry = node
                .logger
                .get_log_entries()
                .filter(|x| x.msg.contains("validated block"))
                .map(|x| x.block_date())
                .last();

            if let Some(log_entry) = log_entry {
                if let Some(last_loaded_block) = log_entry {
                    progress_bar
                        .set_message(&format!("bootstrapping... block: {}", last_loaded_block));
                }
            }
            node.check_no_errors_in_log()?;

            for line in node.logger.get_lines_with_error() {
                progress_bar.set_message(&format!("Error: {}", line));
            }

            if stats.state == NodeState::Running {
                progress_bar.set_message(&format!("bootstrapped succesfully."));
                progress_bar.inc(3);
                return Ok(benchmark.stop());
            }

            thread::sleep(std::time::Duration::from_secs(2));
        }
    }

    fn start_node(
        &self,
        temp_dir: &TempDir,
        storage_folder_name: PathBuf,
    ) -> Result<JormungandrProcess, ClientLoadError> {
        self.copy_initial_storage_if_used(storage_folder_name, temp_dir);

        let config = ConfigurationBuilder::new()
            .with_trusted_peers(vec![self.config.trusted_peer()])
            .with_block_hash(self.config.block0_hash.to_string())
            .with_storage(&temp_dir.child(storage_folder_name.to_string()))
            .build(temp_dir);

        Starter::new().config(config).passive().start_async()
    }

    fn start_node(
        &self,
        temp_dir: &TempDir,
        duration: u32,
        multi_progress: &MultiProgress,
    ) -> Result<thread::JoinHandle<Result<SpeedBenchmarkFinish, ClientLoadError>>, ClientLoadError>
    {
        let iteration = 1;
        let storage_folder_name = self.get_folder_name(iteration);

        let spinner_style = ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{prefix:.bold.dim} {spinner} {wide_msg}");

        let progress_bar = multi_progress.add(ProgressBar::new(3));
        progress_bar.set_style(spinner_style.clone());
        progress_bar.set_prefix(&format!("[Node: {}, iter: {}]", id, iteration));
        progress_bar.set_message(&format!("started..."));
        thread::sleep(std::time::Duration::from_secs(2));
        progress_bar.set_message(&format!("initializing..."));

        let mut benchmarks = vec![];
        let timer = Instant::new();
        let mut node = self.start_node(temp_dir, storage_folder_name)?;
        let mut benchmark = benchmark_speed(&storage_folder_name).no_target().start();

        Ok(thread::spawn(move || {
            loop {
                let benchmark_result = self.wait_for_bootstrap_phase_completed(node)?;

                match benchmark_result {
                    Some(benchmark) => benchmarks.push(benchmark),

                    // if there are no new benchmarks it means that test finished
                    None => return Ok(benchmarks),
                };

                node.shutdown();
                thread::sleep_ms(self.config.pace * 1_000);
                node = self.start_node(temp_dir, storage_folder_name)?;
            }
        }))
    }
}

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

    fn copy_initial_storage_if_used(&self, storage_folder_name: &PathBuf, temp_dir: &TempDir) {
        if let Some(storage) = &self.config.initial_storage {
            let client_storage = temp_dir
                .child(storage_folder_name.to_string())
                .path()
                .into();
            if client_storage.exists() {
                fs::remove_dir_all(&client_storage);
            }
            fs::create_dir(&client_storage).expect("cannot create client storage");
            file_utils::copy_folder(storage, &client_storage, true);
        }
    }

    fn start_node(
        &self,
        temp_dir: &TempDir,
        id: u32,
        iteration: u32,
        multi_progress: &MultiProgress,
    ) -> Result<thread::JoinHandle<Result<SpeedBenchmarkFinish, ClientLoadError>>, ClientLoadError>
    {
        let storage_folder_name = self.get_folder_name();
        self.copy_initial_storage_if_used(storage_folder_name, temp_dir);

        let spinner_style = ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{prefix:.bold.dim} {spinner} {wide_msg}");

        let progress_bar = multi_progress.add(ProgressBar::new(3));
        progress_bar.set_style(spinner_style.clone());
        progress_bar.set_prefix(&format!("[Node: {}, iter: {}]", id, iteration));
        progress_bar.set_message(&format!("started..."));
        thread::sleep(std::time::Duration::from_secs(2));
        progress_bar.set_message(&format!("initializing..."));

        let config = ConfigurationBuilder::new()
            .with_trusted_peers(vec![self.config.trusted_peer()])
            .with_block_hash(self.config.block0_hash.to_string())
            .with_storage(&temp_dir.child(storage_folder_name.to_string()))
            .build(temp_dir);

        let benchmark = benchmark_speed(&storage_folder_name).no_target().start();

        let node = Starter::new()
            .config(config)
            .jormungandr_app(PathBuf::from_str("jormungandr").unwrap())
            .passive()
            .start_async()?;

        Ok(thread::spawn(move || loop {
            let stats = node.rest().stats()?;
            let log_entry = node
                .logger
                .get_log_entries()
                .filter(|x| x.msg.contains("validated block"))
                .map(|x| x.block_date())
                .last();

            if let Some(log_entry) = log_entry {
                if let Some(last_loaded_block) = log_entry {
                    progress_bar
                        .set_message(&format!("bootstrapping... block: {}", last_loaded_block));
                }
            }
            node.check_no_errors_in_log()?;

            for line in node.logger.get_lines_with_error() {
                progress_bar.set_message(&format!("Error: {}", line));
            }

            if stats.state == NodeState::Running {
                progress_bar.set_message(&format!("bootstrapped succesfully."));
                progress_bar.inc(3);
                return Ok(benchmark.stop());
            }

            thread::sleep(std::time::Duration::from_secs(2));
        }))
    }

    pub fn run(&self) -> Result<(), ClientLoadError> {
        let m = MultiProgress::new();
        let mut results = vec![];

        for iter in 1..=self.sync_iteration {
            println!("Iteration {}", iter);

            let mut handles = vec![];
            let mut temp_dirs = vec![];

            for client_id in 1..=self.config.count {
                let temp_dir = TempDir::new().unwrap();
                handles.push(self.start_node(&temp_dir, client_id, iter, &m)?);
                temp_dirs.push(temp_dir);
            }
            m.join_and_clear().unwrap();

            for handle in handles {
                results.push(
                    handle
                        .join()
                        .map_err(|_| ClientLoadError::InternalClientError)?,
                );
            }
        }

        if self.config.measure {
            for result in results {
                if let Ok(result) = &result {
                    println!("{}", result);
                }
            }
        }
        Ok(())
    }
}
