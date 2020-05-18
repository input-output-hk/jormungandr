mod commands;
pub use commands::{get_command, JormungandrStarterCommands};

use super::ConfigurationBuilder;
use crate::common::{
    configuration::{get_jormungandr_app, jormungandr_config::JormungandrConfig},
    file_utils,
    jcli_wrapper::jcli_commands,
    jormungandr::{logger::JormungandrLogger, process::JormungandrProcess},
    process_assert,
    process_utils::{self, output_extensions::ProcessOutput, ProcessError},
};
use jormungandr_testing_utils::testing::{
    network_builder::LeadershipMode, SpeedBenchmarkDef, SpeedBenchmarkRun,
};
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

pub trait StartupVerification {
    fn if_stopped(&self) -> bool;
    fn if_succeed(&self) -> bool;
}

#[derive(Clone, Debug)]
pub struct RestStartupVerification {
    config: JormungandrConfig,
}

impl RestStartupVerification {
    pub fn new(config: JormungandrConfig) -> Self {
        RestStartupVerification { config }
    }
}

impl StartupVerification for RestStartupVerification {
    fn if_stopped(&self) -> bool {
        let logger = JormungandrLogger::new(self.config.log_file_path().unwrap().clone());
        logger.contains_error().unwrap_or_else(|_| false)
    }

    fn if_succeed(&self) -> bool {
        let output = jcli_commands::get_rest_stats_command(&self.config.get_node_address())
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
pub struct LogStartupVerification {
    config: JormungandrConfig,
}

impl LogStartupVerification {
    pub fn new(config: JormungandrConfig) -> Self {
        LogStartupVerification { config }
    }
}

impl StartupVerification for LogStartupVerification {
    fn if_stopped(&self) -> bool {
        let logger = JormungandrLogger::new(self.config.log_file_path().unwrap().clone());
        logger.contains_error().unwrap_or_else(|_| false)
    }

    fn if_succeed(&self) -> bool {
        let logger = JormungandrLogger::new(self.config.log_file_path().unwrap().clone());
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
    config: Option<JormungandrConfig>,
    benchmark: Option<SpeedBenchmarkDef>,
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

    pub fn start_with_fail_in_logs(
        &mut self,
        expected_msg_in_logs: &str,
    ) -> Result<(), StartupError> {
        let config = self.build_configuration();
        let start = Instant::now();
        let _process = self.start_process(&config);

        loop {
            let logger = JormungandrLogger::new(config.log_file_path().unwrap().clone());
            if start.elapsed() > self.timeout {
                return Err(StartupError::EntryNotFoundInLogs {
                    entry: expected_msg_in_logs.to_string(),
                    log_content: logger.get_log_content(),
                });
            }
            process_utils::sleep(self.sleep);
            if logger
                .raw_log_contains_any_of(&[expected_msg_in_logs])
                .unwrap_or_else(|_| false)
            {
                return Ok(());
            }
        }
    }

    fn start_process(&self, config: &JormungandrConfig) -> Child {
        println!("Starting node");
        println!(
            "Blockchain configuration: {:?}",
            config.block0_configuration()
        );
        println!(
            "Node settings configuration: {}",
            file_utils::read_file(&config.node_config_path())
        );

        let mut command = get_command(
            &config.clone().into(),
            get_jormungandr_app(),
            self.role,
            self.from_genesis,
        );

        println!("Bootstrapping...");

        command
            .spawn()
            .expect("failed to execute 'start jormungandr node'")
    }

    pub fn start_async(&mut self) -> Result<JormungandrProcess, StartupError> {
        let config = self.build_configuration();
        println!("{:?}", config.log_file_path());
        Ok(JormungandrProcess::from_config(
            self.start_process(&config),
            config,
            self.alias.clone(),
        ))
    }

    pub fn start(&mut self) -> Result<JormungandrProcess, StartupError> {
        let mut config = self.build_configuration();
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
                    retry_counter = retry_counter - 1;
                }
            }

            if retry_counter < 0 {
                panic!("Jormungandr node cannot start due despite retry attempts. see logs for more details");
            }
        }
    }

    pub fn start_fail(&mut self, expected_msg: &str) {
        let config = self.build_configuration();
        let command = get_command(
            &config.into(),
            get_jormungandr_app(),
            self.role,
            self.from_genesis,
        );
        process_assert::assert_process_failed_and_matches_message(command, &expected_msg);
    }

    fn if_succeed(&self, config: &JormungandrConfig) -> bool {
        match self.verification_mode {
            StartupVerificationMode::Rest => {
                RestStartupVerification::new(config.clone()).if_succeed()
            }
            StartupVerificationMode::Log => {
                LogStartupVerification::new(config.clone()).if_succeed()
            }
        }
    }

    fn if_stopped(&self, config: &JormungandrConfig) -> bool {
        match self.verification_mode {
            StartupVerificationMode::Rest => {
                RestStartupVerification::new(config.clone()).if_stopped()
            }
            StartupVerificationMode::Log => {
                LogStartupVerification::new(config.clone()).if_stopped()
            }
        }
    }

    fn custom_errors_found(&self, config: &JormungandrConfig) -> Result<(), StartupError> {
        let log_file_path = config
            .log_file_path()
            .expect("log file logger has to be defined")
            .clone();
        let logger = JormungandrLogger::new(log_file_path);
        let port_occupied_msgs = ["error 87", "error 98", "panicked at 'Box<Any>'"];
        match logger
            .raw_log_contains_any_of(&port_occupied_msgs)
            .unwrap_or_else(|_| false)
        {
            true => Err(StartupError::PortAlreadyInUse),
            false => Ok(()),
        }
    }

    fn verify_is_up(
        &self,
        process: Child,
        config: &JormungandrConfig,
    ) -> Result<JormungandrProcess, StartupError> {
        let start = Instant::now();
        let log_file_path = config
            .log_file_path()
            .expect("log file logger has to be defined")
            .clone();
        let logger = JormungandrLogger::new(log_file_path.clone());
        loop {
            if start.elapsed() > self.timeout {
                return Err(StartupError::Timeout {
                    timeout: self.timeout.as_secs(),
                    log_content: file_utils::read_file(&log_file_path),
                });
            }
            if self.if_succeed(config) {
                println!("jormungandr is up");
                return Ok(JormungandrProcess::from_config(
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

pub fn restart_jormungandr_node(process: JormungandrProcess, role: Role) -> JormungandrProcess {
    let config = process.config.clone();
    let alias = process.alias().to_string().clone();
    std::mem::drop(process);

    Starter::new()
        .config(config)
        .alias(alias)
        .role(role)
        .start()
        .expect("Jormungandr restart failed")
}
