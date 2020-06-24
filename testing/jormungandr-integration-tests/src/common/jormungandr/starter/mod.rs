mod commands;
pub use commands::{get_command, CommandBuilder};

use super::ConfigurationBuilder;
use crate::common::{
    configuration::{get_jormungandr_app, jormungandr_config::JormungandrParams, TestConfig},
    file_utils,
    jcli_wrapper::jcli_commands,
    jormungandr::{logger::JormungandrLogger, process::JormungandrProcess},
    legacy::{self, LegacyConfigConverter, LegacyConfigConverterError},
    process_assert,
    process_utils::{self, output_extensions::ProcessOutput, ProcessError},
};
use jormungandr_lib::interfaces::NodeConfig;
use jormungandr_testing_utils::testing::{
    network_builder::LeadershipMode, SpeedBenchmarkDef, SpeedBenchmarkRun,
};

use assert_fs::{fixture::FixtureError, TempDir};
use serde::Serialize;
use std::fmt::Debug;
use std::path::PathBuf;
use std::process::Stdio;
use std::{
    process::Child,
    time::{Duration, Instant},
};

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
    #[error("error(s) in log detected: {log_content}")]
    ErrorInLogsFound { log_content: String },
    #[error("error(s) in log detected: port already in use")]
    PortAlreadyInUse,
    #[error("expected message not found: {entry} in logs: {log_content}")]
    EntryNotFoundInLogs { entry: String, log_content: String },
}

const DEFAULT_SLEEP_BETWEEN_ATTEMPTS: u64 = 2;
const DEFAULT_MAX_ATTEMPTS: u64 = 6;

#[derive(Clone, Debug, Copy)]
pub enum StartupVerificationMode {
    Rest,
    Log,
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

#[derive(Clone, Debug, Copy)]
pub enum Role {
    Passive,
    Leader,
}

impl From<LeadershipMode> for Role {
    fn from(leadership_mode: LeadershipMode) -> Self {
        match leadership_mode {
            LeadershipMode::Leader => Self::Leader,
            LeadershipMode::Passive => Self::Passive,
        }
    }
}

impl From<LeadershipMode> for FromGenesis {
    fn from(leadership_mode: LeadershipMode) -> Self {
        match leadership_mode {
            LeadershipMode::Leader => FromGenesis::File,
            LeadershipMode::Passive => FromGenesis::Hash,
        }
    }
}

trait StartupVerification {
    fn if_stopped(&self) -> bool;
    fn if_succeed(&self) -> bool;
}

#[derive(Clone, Debug)]
struct RestStartupVerification<'a, Conf> {
    config: &'a JormungandrParams<Conf>,
}

impl<'a, Conf> RestStartupVerification<'a, Conf> {
    pub fn new(config: &'a JormungandrParams<Conf>) -> Self {
        RestStartupVerification { config }
    }
}

impl<'a, Conf: TestConfig> StartupVerification for RestStartupVerification<'a, Conf> {
    fn if_stopped(&self) -> bool {
        let logger = JormungandrLogger::new(self.config.log_file_path());
        logger.contains_error().unwrap_or_else(|_| false)
    }

    fn if_succeed(&self) -> bool {
        let output = jcli_commands::get_rest_stats_command(&self.config.rest_uri())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
            .wait_with_output()
            .expect("failed to execute get_rest_stats command");

        let content_result = output.try_as_single_node_yaml();
        if content_result.is_err() {
            return false;
        }

        match content_result.unwrap().get("uptime") {
            Some(uptime) => {
                uptime
                    .parse::<i32>()
                    .expect(&format!("Cannot parse uptime {}", uptime.to_string()))
                    > 2
            }
            None => false,
        }
    }
}

#[derive(Clone, Debug)]
struct LogStartupVerification<'a, Conf> {
    config: &'a JormungandrParams<Conf>,
}

impl<'a, Conf> LogStartupVerification<'a, Conf> {
    pub fn new(config: &'a JormungandrParams<Conf>) -> Self {
        LogStartupVerification { config }
    }
}

impl<'a, Conf: TestConfig> StartupVerification for LogStartupVerification<'a, Conf> {
    fn if_stopped(&self) -> bool {
        let logger = JormungandrLogger::new(self.config.log_file_path());
        logger.contains_error().unwrap_or_else(|_| false)
    }

    fn if_succeed(&self) -> bool {
        let logger = JormungandrLogger::new(self.config.log_file_path());
        logger
            .contains_message("genesis block fetched")
            .unwrap_or_else(|_| false)
    }
}

pub struct Starter {
    timeout: Duration,
    jormungandr_app_path: PathBuf,
    sleep: u64,
    role: Role,
    alias: String,
    from_genesis: FromGenesis,
    verification_mode: StartupVerificationMode,
    on_fail: OnFail,
    temp_dir: Option<TempDir>,
    legacy: Option<legacy::Version>,
    config: Option<JormungandrParams>,
    benchmark: Option<SpeedBenchmarkDef>,
}

impl Default for Starter {
    fn default() -> Self {
        Self::new()
    }
}

impl Starter {
    pub fn new() -> Self {
        Starter {
            timeout: Duration::from_secs(300),
            sleep: 2,
            alias: "".to_owned(),
            role: Role::Leader,
            from_genesis: FromGenesis::File,
            verification_mode: StartupVerificationMode::Rest,
            on_fail: OnFail::RetryUnlimitedOnPortOccupied,
            temp_dir: None,
            legacy: None,
            config: None,
            benchmark: None,
            jormungandr_app_path: get_jormungandr_app(),
        }
    }

    pub fn alias(&mut self, alias: String) -> &mut Self {
        self.alias = alias;
        self
    }

    pub fn jormungandr_app(&mut self, path: PathBuf) -> &mut Self {
        self.jormungandr_app_path = path;
        self
    }

    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = timeout;
        self
    }

    pub fn passive(&mut self) -> &mut Self {
        self.role = Role::Passive;
        self
    }

    pub fn benchmark(&mut self, name: &str) -> &mut Self {
        self.benchmark = Some(SpeedBenchmarkDef::new(name.to_owned()));
        self
    }

    pub fn role(&mut self, role: Role) -> &mut Self {
        self.role = role;
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

    pub fn legacy(&mut self, version: legacy::Version) -> &mut Self {
        self.legacy = Some(version);
        self
    }

    pub fn config(&mut self, config: JormungandrParams) -> &mut Self {
        self.config = Some(config);
        self
    }

    pub fn temp_dir(&mut self, temp_dir: TempDir) -> &mut Self {
        self.temp_dir = Some(temp_dir);
        self
    }

    fn build_configuration(
        &mut self,
    ) -> Result<(JormungandrParams, Option<TempDir>), StartupError> {
        let optional_temp_dir = self.temp_dir.take();
        match &self.config {
            Some(params) => Ok((params.clone(), optional_temp_dir)),
            None => {
                let temp_dir = optional_temp_dir.map_or_else(|| TempDir::new(), Ok)?;
                let params = ConfigurationBuilder::new().build(&temp_dir);
                Ok((params, Some(temp_dir)))
            }
        }
    }

    pub fn start_benchmark_run(&self) -> Option<SpeedBenchmarkRun> {
        match &self.benchmark {
            Some(benchmark_def) => Some(benchmark_def.clone().target(self.timeout).start()),
            None => None,
        }
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
        if let Some(version) = self.legacy {
            ConfiguredStarter::legacy(self, version, params, temp_dir)?
                .start_with_fail_in_logs(expected_msg_in_logs)
        } else {
            ConfiguredStarter::new(self, params, temp_dir)
                .start_with_fail_in_logs(expected_msg_in_logs)
        }
    }

    pub fn start_async(&mut self) -> Result<JormungandrProcess, StartupError> {
        let (params, temp_dir) = self.build_configuration()?;
        if let Some(version) = self.legacy {
            ConfiguredStarter::legacy(self, version, params, temp_dir)?.start_async()
        } else {
            ConfiguredStarter::new(self, params, temp_dir).start_async()
        }
    }

    pub fn start(&mut self) -> Result<JormungandrProcess, StartupError> {
        let (params, temp_dir) = self.build_configuration()?;
        let benchmark = self.start_benchmark_run();
        let process = if let Some(version) = self.legacy {
            ConfiguredStarter::legacy(self, version, params, temp_dir)?.start()?
        } else {
            ConfiguredStarter::new(self, params, temp_dir).start()?
        };
        self.finish_benchmark(benchmark);
        Ok(process)
    }

    pub fn start_fail(&mut self, expected_msg: &str) {
        let (params, temp_dir) = self.build_configuration().unwrap();
        if let Some(version) = self.legacy {
            ConfiguredStarter::legacy(self, version, params, temp_dir)
                .unwrap()
                .start_fail(expected_msg)
        } else {
            ConfiguredStarter::new(self, params, temp_dir).start_fail(expected_msg)
        }
    }
}

struct ConfiguredStarter<'a, Conf> {
    starter: &'a Starter,
    params: JormungandrParams<Conf>,
    temp_dir: Option<TempDir>,
}

impl<'a> ConfiguredStarter<'a, NodeConfig> {
    fn new(
        starter: &'a Starter,
        params: JormungandrParams<NodeConfig>,
        temp_dir: Option<TempDir>,
    ) -> Self {
        ConfiguredStarter {
            starter,
            params,
            temp_dir,
        }
    }
}

impl<'a> ConfiguredStarter<'a, legacy::NodeConfig> {
    fn legacy(
        starter: &'a Starter,
        version: legacy::Version,
        params: JormungandrParams<NodeConfig>,
        temp_dir: Option<TempDir>,
    ) -> Result<Self, StartupError> {
        let params = LegacyConfigConverter::new(version).convert(params)?;
        Ok(ConfiguredStarter {
            starter,
            temp_dir,
            params,
        })
    }
}

impl<'a, Conf> ConfiguredStarter<'a, Conf>
where
    Conf: TestConfig + Serialize + Debug,
{
    fn start_with_fail_in_logs(self, expected_msg_in_logs: &str) -> Result<(), StartupError> {
        let start = Instant::now();
        let _process = self.start_process();

        loop {
            let log_file_path = self.params.log_file_path();
            let logger = JormungandrLogger::new(log_file_path);
            if start.elapsed() > self.starter.timeout {
                return Err(StartupError::EntryNotFoundInLogs {
                    entry: expected_msg_in_logs.to_string(),
                    log_content: logger.get_log_content(),
                });
            }
            process_utils::sleep(self.starter.sleep);
            if logger
                .raw_log_contains_any_of(&[expected_msg_in_logs])
                .unwrap_or_else(|_| false)
            {
                return Ok(());
            }
        }
    }

    fn start_process(&self) -> Child {
        println!("Starting node");
        println!(
            "Log file: {}",
            self.params.log_file_path().to_string_lossy()
        );
        println!("Blockchain configuration:");
        println!("{:#?}", self.params.block0_configuration());
        println!("Node settings configuration:");
        println!("{:#?}", self.params.node_config());

        let mut command = get_command(
            &self.params,
            self.starter.jormungandr_app_path.clone(),
            self.starter.role,
            self.starter.from_genesis,
        );

        println!("Bootstrapping...");

        command
            .spawn()
            .expect("failed to execute 'start jormungandr node'")
    }

    fn start_async(self) -> Result<JormungandrProcess, StartupError> {
        Ok(JormungandrProcess::from_config(
            self.start_process(),
            &self.params,
            self.temp_dir,
            self.starter.alias.clone(),
        ))
    }

    fn start(mut self) -> Result<JormungandrProcess, StartupError> {
        let mut retry_counter = 1;
        loop {
            let process = self.start_process();
            let mut jormungandr = JormungandrProcess::from_config(
                process,
                &self.params,
                self.temp_dir.take(),
                self.starter.alias.clone(),
            );

            match (self.verify_is_up(), self.starter.on_fail) {
                (Ok(()), _) => {
                    return Ok(jormungandr);
                }

                (
                    Err(StartupError::PortAlreadyInUse { .. }),
                    OnFail::RetryUnlimitedOnPortOccupied,
                ) => {
                    println!(
                        "Port already in use error detected. Retrying with different port... "
                    );
                    self.params.refresh_instance_params();
                    let path = self.params.log_file_path();
                    std::fs::remove_file(path).unwrap_or_else(|e| {
                        println!(
                            "Failed to remove log file {}: {}",
                            path.to_string_lossy(),
                            e
                        );
                    })
                }
                (Err(err), OnFail::Panic) => {
                    panic!(format!(
                        "Jormungandr node cannot start due to error: {}",
                        err
                    ));
                }
                (Err(err), _) => {
                    println!(
                        "Jormungandr failed to start due to error {}. Retrying... ",
                        err
                    );
                    retry_counter -= 1;
                }
            }

            if retry_counter < 0 {
                panic!("Jormungandr node cannot start due despite retry attempts. see logs for more details");
            }

            self.temp_dir = jormungandr.steal_temp_dir();
        }
    }

    fn start_fail(self, expected_msg: &str) {
        let command = get_command(
            &self.params,
            &self.starter.jormungandr_app_path,
            self.starter.role,
            self.starter.from_genesis,
        );
        process_assert::assert_process_failed_and_matches_message(command, &expected_msg);
    }

    fn if_succeed(&self) -> bool {
        match self.starter.verification_mode {
            StartupVerificationMode::Rest => {
                RestStartupVerification::new(&self.params).if_succeed()
            }
            StartupVerificationMode::Log => LogStartupVerification::new(&self.params).if_succeed(),
        }
    }

    fn if_stopped(&self) -> bool {
        match self.starter.verification_mode {
            StartupVerificationMode::Rest => {
                RestStartupVerification::new(&self.params).if_stopped()
            }
            StartupVerificationMode::Log => LogStartupVerification::new(&self.params).if_stopped(),
        }
    }

    fn custom_errors_found(&self) -> Result<(), StartupError> {
        let log_file_path = self.params.log_file_path();
        if !log_file_path.exists() {
            // Still too early in the startup phase
            return Ok(());
        }
        let logger = JormungandrLogger::new(log_file_path);
        let port_occupied_msgs = ["error 87", "error 98", "panicked at 'Box<Any>'"];
        if logger
            .raw_log_contains_any_of(&port_occupied_msgs)
            .unwrap_or_else(|_| false)
        {
            Err(StartupError::PortAlreadyInUse)
        } else {
            Ok(())
        }
    }

    fn verify_is_up(&self) -> Result<(), StartupError> {
        let start = Instant::now();
        let log_file_path = self.params.log_file_path();
        let logger = JormungandrLogger::new(log_file_path);
        loop {
            if start.elapsed() > self.starter.timeout {
                return Err(StartupError::Timeout {
                    timeout: self.starter.timeout.as_secs(),
                    log_content: file_utils::read_file(&log_file_path),
                });
            }
            if self.if_succeed() {
                println!("jormungandr is up");
                return Ok(());
            }
            self.custom_errors_found()?;
            if self.if_stopped() {
                println!("attempt stopped due to error signal recieved");
                logger.print_raw_log();
                return Err(StartupError::ErrorInLogsFound {
                    log_content: file_utils::read_file(&log_file_path),
                });
            }
            process_utils::sleep(self.starter.sleep);
        }
    }
}
