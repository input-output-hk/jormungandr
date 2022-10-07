#![allow(dead_code)]

mod commands;
pub use commands::{get_command, CommandBuilder};

mod params;
mod testing_directory;

use crate::{
    jormungandr::{legacy::LegacyConfigError, JormungandrError, JormungandrProcess, RestError},
    testing::{configuration::get_jormungandr_app, SpeedBenchmarkDef, SpeedBenchmarkRun},
};
use assert_cmd::assert::OutputAssertExt;
use assert_fs::{fixture::FixtureError, TempDir};
use jormungandr_lib::crypto::hash::Hash;
use jortestkit::process::{self as process_utils, ProcessError};
pub use params::{
    CommunicationParams, ConfigurableNodeConfig, JormungandrBootstrapper, JormungandrParams,
};
use serde::Deserialize;
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};
pub use testing_directory::TestingDirectory;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StartupError {
    #[error("could not start jormungandr due to process issue")]
    JormungandrNotLaunched(#[from] ProcessError),
    #[error("error setting up temporary filesystem fixture")]
    FsFixture(#[from] FixtureError),
    #[error("failed to convert jormungandr configuration to a legacy version")]
    LegacyConfig(#[from] LegacyConfigError),
    #[error("node wasn't properly bootstrap after {timeout} s. Log file: {log_content}")]
    Timeout { timeout: u64, log_content: String },
    #[error("error(s) while starting")]
    JormungandrError(#[from] JormungandrError),
    #[error("expected message not found: {entry} in logs: {log_content}")]
    EntryNotFoundInLogs { entry: String, log_content: String },
    #[error("too many failures while attempting to start jormungandr")]
    TooManyAttempts,
    #[error("Block0 hash is not valid")]
    InvalidBlock0Hash(#[from] chain_crypto::hash::Error),
    #[error("Process exited with status {0}")]
    ProcessExited(ExitStatus),
    #[error("Cannot get rest status")]
    CannotGetRestStatus(#[from] RestError),
    #[error("start params not defined")]
    StartParamsNotDefined,
    #[error(transparent)]
    Params(#[from] crate::jormungandr::starter::params::Error),
}

pub enum StartupVerificationMode {
    Log,
    Rest,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FaketimeConfig {
    /// Clock drift (1 = no drift, 2 = double speed)
    pub drift: f32,
    /// Offset from the real clock in seconds
    pub offset: i32,
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LeadershipMode {
    Leader,
    Passive,
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PersistenceMode {
    Persistent,
    InMemory,
}

#[derive(Debug, Clone)]
pub enum NodeBlock0 {
    Hash(Hash),
    File(PathBuf),
}

#[derive(Clone, Debug, Copy)]
pub enum FromGenesis {
    Hash,
    File,
}

#[derive(Clone, Debug, Copy)]
pub enum OnFail {
    RetryOnce,
    Panic,
    RetryUnlimitedOnPortOccupied,
}

pub struct Starter {
    jormungandr_app_path: Option<PathBuf>,
    configured_starter: ConfiguredStarter,
    alias: String,
    temp_dir: Option<TestingDirectory>,
    config: Option<JormungandrParams>,
    benchmark: Option<SpeedBenchmarkDef>,
}

impl Default for Starter {
    fn default() -> Self {
        let alias = "".to_owned();

        Self {
            configured_starter: ConfiguredStarter::new(&alias),
            alias,
            temp_dir: None,
            config: None,
            benchmark: None,
            jormungandr_app_path: None,
        }
    }
}

impl Starter {
    pub fn alias(mut self, alias: String) -> Self {
        self.alias = alias;
        self
    }

    pub fn jormungandr_app(self, path: PathBuf) -> Self {
        self.jormungandr_app_option(&Some(path))
    }

    pub fn jormungandr_app_option(mut self, path: &Option<PathBuf>) -> Self {
        self.jormungandr_app_path = path.clone();
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.configured_starter.timeout = timeout;
        self
    }

    pub fn benchmark(mut self, name: &str) -> Self {
        self.benchmark = Some(SpeedBenchmarkDef::new(name.to_owned()));
        self
    }

    pub fn verify_by(mut self, verification_mode: StartupVerificationMode) -> Self {
        self.configured_starter.verification_mode = verification_mode;
        self
    }

    pub fn on_fail(mut self, on_fail: OnFail) -> Self {
        self.configured_starter.on_fail = on_fail;
        self
    }

    pub fn config(mut self, config: JormungandrParams) -> Self {
        self.config = Some(config);
        self
    }

    pub fn temp_dir(mut self, temp_dir: TempDir) -> Self {
        self.temp_dir = Some(TestingDirectory::from_temp(temp_dir).into_persistent());
        self
    }

    pub fn working_dir(mut self, path: &Path) -> Self {
        self.temp_dir = Some(path.to_path_buf().into());
        self
    }

    pub fn testing_dir(mut self, testing_directory: TestingDirectory) -> Self {
        self.temp_dir = Some(testing_directory);
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.configured_starter.verbose = verbose;
        self
    }

    fn get_jormungandr_app_path(&self) -> PathBuf {
        self.jormungandr_app_path
            .clone()
            .unwrap_or_else(get_jormungandr_app)
    }

    pub fn start_benchmark_run(&self) -> Option<SpeedBenchmarkRun> {
        self.benchmark.as_ref().map(|benchmark_def| {
            benchmark_def
                .clone()
                .target(self.configured_starter.timeout)
                .start()
        })
    }

    pub fn start_with_fail_in_logs(
        mut self,
        expected_msg_in_logs: &str,
    ) -> Result<(), StartupError> {
        let app = self.get_jormungandr_app_path();
        let temp_dir = self.temp_dir.take();
        let params = self.config.ok_or(StartupError::StartParamsNotDefined)?;
        let comm = params.comm();
        self.configured_starter.start_with_fail_in_logs(
            comm,
            temp_dir,
            expected_msg_in_logs,
            get_command(&params, &app, params.leadership()),
        )
    }

    pub fn start_async(mut self) -> Result<JormungandrProcess, StartupError> {
        let app = self.get_jormungandr_app_path();
        let temp_dir = self.temp_dir.take();
        let params = self.config.ok_or(StartupError::StartParamsNotDefined)?;
        let comm = params.comm();
        self.configured_starter.start_async(
            comm,
            temp_dir,
            get_command(&params, &app, params.leadership()),
        )
    }

    pub fn start(mut self) -> Result<JormungandrProcess, StartupError> {
        let app = self.get_jormungandr_app_path();
        let temp_dir = self.temp_dir.take();
        let benchmark = self.start_benchmark_run();
        let params = self.config.ok_or(StartupError::StartParamsNotDefined)?;
        let command = get_command(&params, app, params.leadership());
        let process = self.configured_starter.start(params, temp_dir, command)?;
        finish_benchmark(benchmark);
        Ok(process)
    }

    pub fn start_should_fail_with_message(self, expected_msg: &str) -> Result<(), StartupError> {
        let app = self.get_jormungandr_app_path();
        let params = self.config.ok_or(StartupError::StartParamsNotDefined)?;
        let command = get_command(&params, app, params.leadership());
        ConfiguredStarter::new(&self.alias).start_with_fail_in_stderr(command, expected_msg);
        Ok(())
    }
}

fn finish_benchmark(benchmark_run: Option<SpeedBenchmarkRun>) {
    if let Some(benchmark_run) = benchmark_run {
        benchmark_run.stop().print();
    }
}

pub struct ConfiguredStarter {
    alias: String,
    timeout: Duration,
    sleep: u64,
    verification_mode: StartupVerificationMode,
    on_fail: OnFail,
    verbose: bool,
}

impl ConfiguredStarter {
    pub fn new(alias: impl Into<String>) -> Self {
        Self {
            alias: alias.into(),
            timeout: Duration::from_secs(30),
            sleep: 2,
            verification_mode: StartupVerificationMode::Rest,
            on_fail: OnFail::RetryUnlimitedOnPortOccupied,
            verbose: true,
        }
    }

    pub fn start_with_fail_in_logs(
        self,
        comms: CommunicationParams,
        temp_dir: Option<TestingDirectory>,
        expected_msg_in_logs: &str,
        command: Command,
    ) -> Result<(), StartupError> {
        let sleep_duration = self.sleep;
        let timeout = self.timeout;

        let start = Instant::now();
        let process = self.start_async(comms, temp_dir, command)?;

        loop {
            if start.elapsed() > timeout {
                return Err(StartupError::EntryNotFoundInLogs {
                    entry: expected_msg_in_logs.to_string(),
                    log_content: process.logger.get_log_content(),
                });
            }
            process_utils::sleep(sleep_duration);
            if process.logger.contains_any_of(&[expected_msg_in_logs]) {
                return Ok(());
            }
        }
    }

    pub fn start_with_fail_in_stderr(self, mut command: Command, expected_msg: &str) {
        let verbose = self.verbose;
        crate::cond_println!(verbose, "Running start command: {:?}", command);
        crate::cond_println!(
            verbose,
            "Expecting node to fail with message '{expected_msg}'..."
        );
        command
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn start_process(&self, command: &mut Command) -> Child {
        let verbose = self.verbose;
        crate::cond_println!(verbose, "Running start command: {:?}", command);
        crate::cond_println!(verbose, "Bootstrapping...");
        command
            .spawn()
            .expect("failed to execute 'start jormungandr node'")
    }

    pub fn start_async(
        self,
        comms: CommunicationParams,
        temp_dir: Option<TestingDirectory>,
        mut command: Command,
    ) -> Result<JormungandrProcess, StartupError> {
        JormungandrProcess::new(
            self.start_process(&mut command),
            comms,
            temp_dir,
            self.alias.clone(),
        )
    }

    pub fn start(
        self,
        mut params: JormungandrParams,
        mut temp_dir: Option<TestingDirectory>,
        mut command: Command,
    ) -> Result<JormungandrProcess, StartupError> {
        let mut retry_counter = 1;
        loop {
            let process = self.start_process(&mut command);

            let mut jormungandr = JormungandrProcess::new(
                process,
                params.comm(),
                temp_dir.take(),
                self.alias.clone(),
            )?;

            match (
                jormungandr.wait_for_bootstrap(&self.verification_mode, self.timeout),
                self.on_fail,
            ) {
                (Ok(()), _) => {
                    crate::cond_println!(self.verbose, "jormungandr is up");
                    return Ok(jormungandr);
                }

                (
                    Err(StartupError::JormungandrError(JormungandrError::PortAlreadyInUse)),
                    OnFail::RetryUnlimitedOnPortOccupied,
                ) => {
                    crate::cond_println!(
                        self.verbose,
                        "Port already in use error detected. Retrying with different port... "
                    );
                    params.refresh_instance_params();
                }
                (Err(err), OnFail::Panic) => {
                    panic!("Jormungandr node cannot start due to error: {}", err);
                }
                (Err(err), _) => {
                    crate::cond_println!(
                        self.verbose,
                        "Jormungandr failed to start due to error {:?}. Retrying... ",
                        err
                    );
                    retry_counter -= 1;
                }
            }

            if retry_counter < 0 {
                return Err(StartupError::TooManyAttempts);
            }

            temp_dir = jormungandr.steal_temp_dir();
        }
    }
}
