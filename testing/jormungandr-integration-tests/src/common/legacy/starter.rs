use super::BackwardCompatibleConfig;
use super::BackwardCompatibleJormungandr;
use super::Version;
use super::{LegacyConfigConverter, LegacyConfigConverterError};
use crate::common::{
    configuration::JormungandrConfig,
    file_utils,
    jcli_wrapper::jcli_commands,
    jormungandr::{
        logger::JormungandrLogger,
        starter::{get_command, FromGenesis, OnFail, Role},
        ConfigurationBuilder,
    },
    process_utils::{self, output_extensions::ProcessOutput, ProcessError},
};
use jormungandr_testing_utils::testing::{SpeedBenchmarkDef, SpeedBenchmarkRun};
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
    #[error("could not start jormungandr due to process issue")]
    ConfigurationError(#[from] LegacyConfigConverterError),
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

pub struct Starter {
    timeout: Duration,
    jormungandr_app_path: PathBuf,
    sleep: u64,
    role: Role,
    alias: String,
    from_genesis: FromGenesis,
    on_fail: OnFail,
    version: Version,
    config: Option<JormungandrConfig>,
    benchmark: Option<SpeedBenchmarkDef>,
}

impl Starter {
    pub fn new(version: Version, jormungandr_app_path: PathBuf) -> Self {
        Starter {
            timeout: Duration::from_secs(300),
            sleep: 2,
            role: Role::Leader,
            from_genesis: FromGenesis::File,
            on_fail: OnFail::RetryUnlimitedOnPortOccupied,
            version,
            alias: "".to_owned(),
            config: None,
            benchmark: None,
            jormungandr_app_path,
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

    pub fn config(&mut self, config: JormungandrConfig) -> &mut Self {
        self.config = Some(config);
        self
    }

    fn build_configuration(&mut self) -> JormungandrConfig {
        if self.config.is_none() {
            self.config = Some(ConfigurationBuilder::new().build());
        }
        self.config.as_ref().unwrap().clone()
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

    fn start_process(&self, config: &BackwardCompatibleConfig) -> Child {
        println!("Starting legacy node: {}", self.version);
        println!(
            "Blockchain configuration: {:?}",
            &config.block0_configuration
        );
        println!(
            "Node settings configuration: {}",
            file_utils::read_file(&config.node_config_path)
        );

        let mut command = get_command(
            config,
            self.jormungandr_app_path.clone(),
            self.role,
            self.from_genesis,
        );

        println!("Bootstrapping...");

        command
            .spawn()
            .expect("failed to execute 'start jormungandr node'")
    }

    pub fn start(&mut self) -> Result<BackwardCompatibleJormungandr, StartupError> {
        let mut config =
            LegacyConfigConverter::new(self.version.clone()).convert(self.build_configuration())?;
        let benchmark = self.start_benchmark_run();
        let mut retry_counter = 1;
        loop {
            let process = self.start_process(&config);

            match (self.verify_is_up(process, &config), self.on_fail) {
                (Ok(jormungandr_process), _) => {
                    self.finish_benchmark(benchmark);
                    return Ok(jormungandr_process);
                }

                (
                    Err(StartupError::PortAlreadyInUse { .. }),
                    OnFail::RetryUnlimitedOnPortOccupied,
                ) => {
                    println!(
                        "Port already in use error detected. Retrying with different port... "
                    );
                    config.refresh_node_dynamic_params();
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
        }
    }

    fn if_succeed(&self, config: &BackwardCompatibleConfig) -> bool {
        let output = jcli_commands::get_rest_stats_command(&config.get_node_address())
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

    fn if_stopped(&self, config: &BackwardCompatibleConfig) -> bool {
        let log_file_path = config.log_file_path().expect("No log file defined");
        let logger = JormungandrLogger::new(log_file_path);
        logger.contains_error().unwrap_or_else(|_| false)
    }

    fn custom_errors_found(&self, config: &BackwardCompatibleConfig) -> Result<(), StartupError> {
        let log_file_path = config.log_file_path().expect("No log file defined");

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

    fn verify_is_up(
        &self,
        process: Child,
        config: &BackwardCompatibleConfig,
    ) -> Result<BackwardCompatibleJormungandr, StartupError> {
        let start = Instant::now();
        let log_file_path = config.log_file_path().expect("No log file defined");

        let logger = JormungandrLogger::new(log_file_path.clone());
        loop {
            if start.elapsed() > self.timeout {
                println!(
                    "Timeout!: elapsed: '{:?}', waited till: '{:?}' ",
                    start.elapsed(),
                    self.timeout
                );
                return Err(StartupError::Timeout {
                    timeout: self.timeout.as_secs(),
                    log_content: file_utils::read_file(&log_file_path),
                });
            }
            if self.if_succeed(config) {
                println!("jormungandr is up");
                return Ok(BackwardCompatibleJormungandr::from_config(
                    process,
                    config.clone(),
                    self.alias.clone(),
                ));
            }
            self.custom_errors_found(config)?;
            if self.if_stopped(config) {
                println!("attempt stopped due to error signal recieved");
                logger.print_raw_log();
                return Err(StartupError::ErrorInLogsFound {
                    log_content: file_utils::read_file(&log_file_path),
                });
            }
            process_utils::sleep(self.sleep);
        }
    }
}
