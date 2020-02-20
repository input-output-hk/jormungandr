use super::ConfigurationBuilder;
use crate::common::{
    configuration::jormungandr_config::JormungandrConfig,
    file_utils,
    jcli_wrapper::jcli_commands,
    jormungandr::{commands, logger::JormungandrLogger, process::JormungandrProcess},
    process_assert,
    process_utils::{self, output_extensions::ProcessOutput, ProcessError},
};
use jormungandr_lib::testing::{SpeedBenchmarkDef, SpeedBenchmarkRun};
use std::{
    process::{Child, Command},
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
}

const DEFAULT_SLEEP_BETWEEN_ATTEMPTS: u64 = 2;
const DEFAULT_MAX_ATTEMPTS: u64 = 6;

#[derive(Clone, Debug, Copy)]
pub enum StartupVerificationMode {
    Rest,
    Log,
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
        let logger = JormungandrLogger::new(self.config.log_file_path.clone());
        logger.contains_error().unwrap_or_else(|_| false)
    }

    fn if_succeed(&self) -> bool {
        let output = process_utils::run_process_and_get_output(
            jcli_commands::get_rest_stats_command(&self.config.get_node_address()),
        );

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
        let logger = JormungandrLogger::new(self.config.log_file_path.clone());
        logger.contains_error().unwrap_or_else(|_| false)
    }

    fn if_succeed(&self) -> bool {
        let logger = JormungandrLogger::new(self.config.log_file_path.clone());
        logger
            .contains_message("genesis block fetched")
            .unwrap_or_else(|_| false)
    }
}

pub struct Starter {
    timeout: Duration,
    sleep: u64,
    role: Role,
    verification_mode: StartupVerificationMode,
    explorer_enabled: bool,
    on_fail: OnFail,
    config: Option<JormungandrConfig>,
    benchmark: Option<SpeedBenchmarkDef>,
}

impl Starter {
    pub fn new() -> Self {
        Starter {
            timeout: Duration::from_secs(300),
            sleep: 2,
            role: Role::Leader,
            verification_mode: StartupVerificationMode::Rest,
            explorer_enabled: false,
            on_fail: OnFail::RetryUnlimitedOnPortOccupied,
            config: None,
            benchmark: None,
        }
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

    pub fn with_explorer(&mut self) -> &mut Self {
        self.explorer_enabled = true;
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

    pub fn start(&mut self) -> Result<JormungandrProcess, StartupError> {
        let mut config = self.build_configuration();
        let benchmark = self.start_benchmark_run();
        let mut retry_counter = 1;
        loop {
            let mut command = self.get_command(&config);
            println!("Starting node with configuration : {:?}", &config);

            let process = command
                .spawn()
                .expect("failed to execute 'start jormungandr node'");

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
        let command = self.get_command(&config);
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
        let logger = JormungandrLogger::new(config.log_file_path.clone());
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
        let logger = JormungandrLogger::new(config.log_file_path.clone());
        loop {
            if start.elapsed() > self.timeout {
                return Err(StartupError::Timeout {
                    timeout: self.timeout.as_secs(),
                    log_content: file_utils::read_file(&config.log_file_path),
                });
            }
            if self.if_succeed(config) {
                println!("jormungandr is up");
                return Ok(JormungandrProcess::from_config(process, config.clone()));
            }
            self.custom_errors_found(config)?;
            if self.if_stopped(config) {
                println!("attempt stopped due to error signal recieved");
                logger.print_raw_log();
                return Err(StartupError::ErrorInLogsFound {
                    log_content: file_utils::read_file(&config.log_file_path),
                });
            }
            process_utils::sleep(self.sleep);
        }
    }

    fn get_command(&self, config: &JormungandrConfig) -> Command {
        match self.role {
            Role::Passive => commands::get_start_jormungandr_as_passive_node_command(
                &config.node_config_path,
                &config.genesis_block_hash,
                &config.log_file_path,
            ),
            Role::Leader => commands::get_start_jormungandr_as_leader_node_command(
                &config.node_config_path,
                &config.genesis_block_path,
                &config.secret_model_paths,
                &config.log_file_path,
            ),
        }
    }
}

pub fn restart_jormungandr_node(process: JormungandrProcess, role: Role) -> JormungandrProcess {
    let config = process.config.clone();
    std::mem::drop(process);

    Starter::new()
        .config(config)
        .role(role)
        .start()
        .expect("Jormungandr restart failed")
}
