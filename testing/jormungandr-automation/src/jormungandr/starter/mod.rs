#![allow(dead_code)]

mod commands;
pub use commands::{get_command, CommandBuilder};

mod testing_directory;
use crate::{
    jormungandr::{
        ConfigurationBuilder, JormungandrError, JormungandrParams, JormungandrProcess,
        LegacyConfigConverter, LegacyConfigConverterError, LegacyNodeConfig, RestError, TestConfig,
        Version,
    },
    testing::{configuration::get_jormungandr_app, SpeedBenchmarkDef, SpeedBenchmarkRun},
};
use assert_cmd::assert::OutputAssertExt;
use assert_fs::{fixture::FixtureError, TempDir};
use chain_impl_mockchain::header::HeaderId;
use jormungandr_lib::interfaces::NodeConfig;
use jortestkit::process::{self as process_utils, ProcessError};
use serde::{Deserialize, Serialize};
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
    LegacyConfigConversion(#[from] LegacyConfigConverterError),
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
    Hash(HeaderId),
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

impl From<LeadershipMode> for FromGenesis {
    fn from(leadership_mode: LeadershipMode) -> Self {
        match leadership_mode {
            LeadershipMode::Leader => FromGenesis::File,
            LeadershipMode::Passive => FromGenesis::Hash,
        }
    }
}

pub struct Starter {
    timeout: Duration,
    jormungandr_app_path: Option<PathBuf>,
    sleep: u64,
    leadership_mode: LeadershipMode,
    alias: String,
    from_genesis: FromGenesis,
    verification_mode: StartupVerificationMode,
    on_fail: OnFail,
    temp_dir: Option<TestingDirectory>,
    legacy: Option<Version>,
    config: Option<JormungandrParams>,
    benchmark: Option<SpeedBenchmarkDef>,
    verbose: bool,
}

impl Default for Starter {
    fn default() -> Self {
        Self::new()
    }
}

impl Starter {
    pub fn new() -> Self {
        Starter {
            timeout: Duration::from_secs(30),
            sleep: 2,
            alias: "".to_owned(),
            leadership_mode: LeadershipMode::Leader,
            from_genesis: FromGenesis::File,
            verification_mode: StartupVerificationMode::Rest,
            on_fail: OnFail::RetryUnlimitedOnPortOccupied,
            temp_dir: None,
            legacy: None,
            config: None,
            benchmark: None,
            jormungandr_app_path: None,
            verbose: true,
        }
    }

    pub fn alias(&mut self, alias: String) -> &mut Self {
        self.alias = alias;
        self
    }

    pub fn jormungandr_app(&mut self, path: PathBuf) -> &mut Self {
        self.jormungandr_app_option(&Some(path))
    }

    pub fn jormungandr_app_option(&mut self, path: &Option<PathBuf>) -> &mut Self {
        self.jormungandr_app_path = path.clone();
        self
    }

    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = timeout;
        self
    }

    pub fn passive(&mut self) -> &mut Self {
        self.leadership_mode = LeadershipMode::Passive;
        self
    }

    pub fn benchmark(&mut self, name: &str) -> &mut Self {
        self.benchmark = Some(SpeedBenchmarkDef::new(name.to_owned()));
        self
    }

    pub fn leadership_mode(&mut self, leadership_mode: LeadershipMode) -> &mut Self {
        self.leadership_mode = leadership_mode;
        self
    }

    pub fn from_genesis_hash(&mut self) -> &mut Self {
        self.from_genesis(FromGenesis::Hash)
    }

    pub fn from_genesis_file(&mut self) -> &mut Self {
        self.from_genesis(FromGenesis::File)
    }

    pub fn from_genesis(&mut self, from_genesis: FromGenesis) -> &mut Self {
        self.from_genesis = from_genesis;
        self
    }

    pub fn verify_by(&mut self, verification_mode: StartupVerificationMode) -> &mut Self {
        self.verification_mode = verification_mode;
        self
    }

    pub fn on_fail(&mut self, on_fail: OnFail) -> &mut Self {
        self.on_fail = on_fail;
        self
    }

    pub fn legacy(&mut self, version: Version) -> &mut Self {
        self.legacy = Some(version);
        self
    }

    pub fn config(&mut self, config: JormungandrParams) -> &mut Self {
        self.config = Some(config);
        self
    }

    pub fn temp_dir(&mut self, temp_dir: TempDir) -> &mut Self {
        self.temp_dir = Some(TestingDirectory::from_temp(temp_dir));
        self
    }

    pub fn working_dir(&mut self, path: &Path) -> &mut Self {
        self.temp_dir = Some(path.to_path_buf().into());
        self
    }

    pub fn testing_dir(&mut self, testing_directory: TestingDirectory) -> &mut Self {
        self.temp_dir = Some(testing_directory);
        self
    }

    pub fn verbose(&mut self, verbose: bool) -> &mut Self {
        self.verbose = verbose;
        self
    }

    pub fn build_configuration(
        &mut self,
    ) -> Result<(JormungandrParams, Option<TestingDirectory>), StartupError> {
        let optional_temp_dir = self.temp_dir.take();
        match &self.config {
            Some(params) => Ok((params.clone(), optional_temp_dir)),
            None => {
                let temp_dir = optional_temp_dir.map_or_else(TestingDirectory::new_temp, Ok)?;
                let params = ConfigurationBuilder::new().build(&temp_dir);
                Ok((params, Some(temp_dir)))
            }
        }
    }

    pub fn start_benchmark_run(&self) -> Option<SpeedBenchmarkRun> {
        self.benchmark
            .as_ref()
            .map(|benchmark_def| benchmark_def.clone().target(self.timeout).start())
    }

    pub fn finish_benchmark(&self, benchmark_run: Option<SpeedBenchmarkRun>) {
        if let Some(benchmark_run) = benchmark_run {
            benchmark_run.stop().print();
        }
    }

    pub fn start_with_fail_in_logs(
        &mut self,
        expected_msg_in_logs: &str,
    ) -> Result<(), StartupError> {
        let (params, temp_dir) = self.build_configuration()?;
        if let Some(version) = self.legacy.as_ref() {
            ConfiguredStarter::legacy(self, version.clone(), params, temp_dir)?
                .start_with_fail_in_logs(expected_msg_in_logs)
        } else {
            ConfiguredStarter::new(self, params, temp_dir)
                .start_with_fail_in_logs(expected_msg_in_logs)
        }
    }

    pub fn start_async(&mut self) -> Result<JormungandrProcess, StartupError> {
        let (params, temp_dir) = self.build_configuration()?;
        if let Some(version) = self.legacy.as_ref() {
            Ok(
                ConfiguredStarter::legacy(self, version.clone(), params, temp_dir)?
                    .start_async()?,
            )
        } else {
            Ok(ConfiguredStarter::new(self, params, temp_dir).start_async()?)
        }
    }

    pub fn start(&mut self) -> Result<JormungandrProcess, StartupError> {
        let (params, temp_dir) = self.build_configuration()?;
        let benchmark = self.start_benchmark_run();
        let process = if let Some(version) = self.legacy.as_ref() {
            ConfiguredStarter::legacy(self, version.clone(), params, temp_dir)?.start()?
        } else {
            ConfiguredStarter::new(self, params, temp_dir).start()?
        };
        self.finish_benchmark(benchmark);
        Ok(process)
    }

    pub fn start_fail(&mut self, expected_msg: &str) {
        let (params, temp_dir) = self.build_configuration().unwrap();
        if let Some(version) = self.legacy.as_ref() {
            ConfiguredStarter::legacy(self, version.clone(), params, temp_dir)
                .unwrap()
                .start_with_fail_in_stderr(expected_msg);
        } else {
            ConfiguredStarter::new(self, params, temp_dir).start_with_fail_in_stderr(expected_msg);
        }
    }
}

pub struct ConfiguredStarter<'a, Conf> {
    starter: &'a Starter,
    params: JormungandrParams<Conf>,
    temp_dir: Option<TestingDirectory>,
}

impl<'a> ConfiguredStarter<'a, NodeConfig> {
    pub fn new(
        starter: &'a Starter,
        params: JormungandrParams<NodeConfig>,
        temp_dir: Option<TestingDirectory>,
    ) -> Self {
        ConfiguredStarter {
            starter,
            params,
            temp_dir,
        }
    }
}

impl<'a> ConfiguredStarter<'a, LegacyNodeConfig> {
    pub fn legacy(
        starter: &'a Starter,
        version: Version,
        params: JormungandrParams<NodeConfig>,
        temp_dir: Option<TestingDirectory>,
    ) -> Result<Self, StartupError> {
        let params = LegacyConfigConverter::new(version).convert(params)?;
        params.write_node_config();
        Ok(ConfiguredStarter {
            starter,
            params,
            temp_dir,
        })
    }
}

macro_rules! cond_println {
    ($cond:expr, $($arg:tt)*) => {
        if $cond {
            println!($($arg)*);
        }
    };
}

impl<'a, Conf> ConfiguredStarter<'a, Conf>
where
    Conf: TestConfig + Serialize + Debug,
{
    fn start_with_fail_in_logs(self, expected_msg_in_logs: &str) -> Result<(), StartupError> {
        let sleep_duration = self.starter.sleep;
        let timeout = self.starter.timeout;

        let start = Instant::now();
        let process = self.start_async()?;

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

    fn start_with_fail_in_stderr(self, expected_msg: &str) {
        get_command(
            &self.params,
            &self
                .starter
                .jormungandr_app_path
                .clone()
                .unwrap_or_else(get_jormungandr_app),
            self.starter.leadership_mode,
            self.starter.from_genesis,
        )
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .assert()
        .failure()
        .stderr(predicates::str::contains(expected_msg));
    }

    fn start_process(&self) -> Child {
        let verbose = self.starter.verbose;
        cond_println!(verbose, "Starting node");
        cond_println!(verbose, "Blockchain configuration:");
        cond_println!(verbose, "{:#?}", self.params.block0_configuration());
        cond_println!(verbose, "Node settings configuration:");
        cond_println!(verbose, "{:#?}", self.params.node_config());

        let mut command = self.command();
        cond_println!(verbose, "Running start command: {:?}", command);
        cond_println!(verbose, "Bootstrapping...");
        command
            .spawn()
            .expect("failed to execute 'start jormungandr node'")
    }

    pub fn command(&self) -> Command {
        get_command(
            &self.params,
            self.starter
                .jormungandr_app_path
                .clone()
                .unwrap_or_else(get_jormungandr_app),
            self.starter.leadership_mode,
            self.starter.from_genesis,
        )
    }

    fn start_async(self) -> Result<JormungandrProcess, StartupError> {
        JormungandrProcess::new(
            self.start_process(),
            self.params.node_config(),
            self.params.block0_configuration().clone(),
            self.temp_dir,
            self.starter.alias.clone(),
        )
    }

    fn start(mut self) -> Result<JormungandrProcess, StartupError> {
        let mut retry_counter = 1;
        loop {
            let process = self.start_process();

            let mut jormungandr = JormungandrProcess::new(
                process,
                self.params.node_config(),
                self.params.block0_configuration().clone(),
                self.temp_dir.take(),
                self.starter.alias.clone(),
            )?;

            match (
                jormungandr
                    .wait_for_bootstrap(&self.starter.verification_mode, self.starter.timeout),
                self.starter.on_fail,
            ) {
                (Ok(()), _) => {
                    cond_println!(self.starter.verbose, "jormungandr is up");
                    return Ok(jormungandr);
                }

                (
                    Err(StartupError::JormungandrError(JormungandrError::PortAlreadyInUse)),
                    OnFail::RetryUnlimitedOnPortOccupied,
                ) => {
                    cond_println!(
                        self.starter.verbose,
                        "Port already in use error detected. Retrying with different port... "
                    );
                    self.params.refresh_instance_params();
                }
                (Err(err), OnFail::Panic) => {
                    panic!("Jormungandr node cannot start due to error: {}", err);
                }
                (Err(err), _) => {
                    cond_println!(
                        self.starter.verbose,
                        "Jormungandr failed to start due to error {:?}. Retrying... ",
                        err
                    );
                    retry_counter -= 1;
                }
            }

            if retry_counter < 0 {
                return Err(StartupError::TooManyAttempts);
            }

            self.temp_dir = jormungandr.steal_temp_dir();
        }
    }
}
