use super::logger::JormungandrLogger;
use crate::common::configuration::jormungandr_config::JormungandrConfig;
use std::path::PathBuf;
use std::process::Child;

#[derive(Debug)]
pub struct JormungandrProcess {
    pub child: Child,
    pub logger: JormungandrLogger,
    pub config: JormungandrConfig,
    description: String,
}

impl JormungandrProcess {
    pub fn from_config(child: Child, config: JormungandrConfig) -> Self {
        JormungandrProcess::new(
            child,
            String::from("Jormungandr node"),
            config.log_file_path.clone(),
            config,
        )
    }

    pub fn new(
        child: Child,
        description: String,
        log_file_path: PathBuf,
        config: JormungandrConfig,
    ) -> Self {
        JormungandrProcess {
            child: child,
            description: String::from("Jormungandr node"),
            logger: JormungandrLogger::new(config.log_file_path.clone()),
            config: config,
        }
    }

    pub fn new(
        child: Child,
        description: String,
        log_file_path: PathBuf,
        config: JormungandrConfig,
    ) -> Self {
        JormungandrProcess {
            child: child,
            description: description,
            logger: JormungandrLogger::new(log_file_path.clone()),
            config: config,
        }
    }

    pub fn assert_no_errors_in_log(&self) {
        let error_lines = self.logger.get_lines_with_error().collect::<Vec<String>>();

        assert_eq!(
            error_lines.len(),
            0,
            "there are some errors in log ({:?}): {:?}",
            self.logger.log_file_path,
            error_lines
        );
    }
}

impl Drop for JormungandrProcess {
    fn drop(&mut self) {
        match self.child.kill() {
            Err(e) => println!("Could not kill {}: {}", self.description, e),
            Ok(_) => println!("Successfully killed {}", self.description),
        }
    }
}
