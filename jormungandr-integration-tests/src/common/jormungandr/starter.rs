extern crate custom_error;

use self::custom_error::custom_error;
use super::ConfigurationBuilder;
use crate::common::{
    configuration::jormungandr_config::JormungandrConfig,
    file_utils,
    jcli_wrapper::jcli_commands,
    jormungandr::{commands, logger::JormungandrLogger, process::JormungandrProcess},
    process_assert,
    process_utils::{self, output_extensions::ProcessOutput, ProcessError},
};

use std::{
    process::{Child, Command},
    time::{Duration, Instant},
};
custom_error! {pub StartupError
    JormungandrNotLaunched{ source: ProcessError } = "could not start jormungandr due to process issue",
    Timeout{ timeout: u64, log_content: String } = "node wasn't properly bootstrap after {timeout} s. Log file: {log_content}",
    ErrorInLogsFound { log_content: String } = "error(s) in log detected: {log_content} ",
    PortAlreadyInUse = "error(s) in log detected: port already in use "
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
    fn stop(&self) -> bool;
    fn success(&self) -> bool;
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
    fn stop(&self) -> bool {
        let logger = JormungandrLogger::new(self.config.log_file_path.clone());
        logger.contains_error().unwrap_or_else(|_| false)
    }

    fn success(&self) -> bool {
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
    fn stop(&self) -> bool {
        let logger = JormungandrLogger::new(self.config.log_file_path.clone());
        logger.contains_error().unwrap_or_else(|_| false)
    }

    fn success(&self) -> bool {
        let logger = JormungandrLogger::new(self.config.log_file_path.clone());
        logger
            .message_logged_multiple_times("initial bootstrap completed", 2)
            .unwrap_or_else(|_| false)
    }
}

pub struct Starter {
    timeout: Duration,
    sleep: u64,
    role: Role,
    verification_mode: StartupVerificationMode,
    on_fail: OnFail,
    config: JormungandrConfig,
}

impl Starter {
    pub fn new() -> Self {
        Starter {
            timeout: Duration::from_secs(300),
            sleep: 2,
            role: Role::Leader,
            verification_mode: StartupVerificationMode::Rest,
            on_fail: OnFail::RetryUnlimitedOnPortOccupied,
            config: ConfigurationBuilder::new().build(),
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

    pub fn role(&mut self, role: Role) -> &mut Self {
        self.role = role;
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
        self.config = config;
        self
    }

    pub fn start(&mut self) -> Result<JormungandrProcess, StartupError> {
        let mut retry_counter = 1;
        loop {
            let mut command = self.get_command(&self.config);
            println!("Starting node with configuration : {:?}", &self.config);

            let process = command
                .spawn()
                .expect("failed to execute 'start jormungandr node'");

            match (self.verify_is_up(process), self.on_fail) {
                (Ok(jormungandr_process), _) => return Ok(jormungandr_process),

                (
                    Err(StartupError::PortAlreadyInUse { .. }),
                    OnFail::RetryUnlimitedOnPortOccupied,
                ) => {
                    println!(
                        "Port already in use error detected. Retrying with different port... "
                    );
                    self.config.refresh_node_dynamic_params();
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

    pub fn start_fail(&self, expected_msg: &str) {
        let command = self.get_command(&self.config);
        process_assert::assert_process_failed_and_matches_message(command, &expected_msg);
    }

    fn success(&self) -> bool {
        match self.verification_mode {
            StartupVerificationMode::Rest => {
                RestStartupVerification::new(self.config.clone()).success()
            }
            StartupVerificationMode::Log => {
                LogStartupVerification::new(self.config.clone()).success()
            }
        }
    }

    fn stop(&self) -> bool {
        match self.verification_mode {
            StartupVerificationMode::Rest => {
                RestStartupVerification::new(self.config.clone()).stop()
            }
            StartupVerificationMode::Log => LogStartupVerification::new(self.config.clone()).stop(),
        }
    }

    fn custom_errors_found(&self) -> Result<(), StartupError> {
        let logger = JormungandrLogger::new(self.config.log_file_path.clone());
        //Can not resume socket accept process: The parameter is incorrect. (os error 87)
        match logger
            .contains_message("error 87")
            .unwrap_or_else(|_| false)
        {
            true => Err(StartupError::PortAlreadyInUse),
            false => Ok(()),
        }
    }

    fn verify_is_up(&self, process: Child) -> Result<JormungandrProcess, StartupError> {
        let start = Instant::now();
        let logger = JormungandrLogger::new(self.config.log_file_path.clone());
        loop {
            if start.elapsed() > self.timeout {
                return Err(StartupError::Timeout {
                    timeout: self.timeout.as_secs(),
                    log_content: file_utils::read_file(&self.config.log_file_path),
                });
            }
            if self.success() {
                return Ok(JormungandrProcess::from_config(
                    process,
                    self.config.clone(),
                ));
            }
            if self.stop() {
                logger.print_raw_log();
                return Err(StartupError::ErrorInLogsFound {
                    log_content: file_utils::read_file(&self.config.log_file_path),
                });
            }
            self.custom_errors_found()?;
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
                &config.secret_model_path,
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
