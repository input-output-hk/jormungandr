use super::logger::JormungandrLogger;
use std::path::PathBuf;
use std::process::Child;

#[derive(Debug)]
pub struct JormungandrProcess {
    pub child: Child,
    pub logger: JormungandrLogger,
    description: String,
}

impl JormungandrProcess {
    pub fn new(child: Child, description: String, log_file_path: PathBuf) -> Self {
        JormungandrProcess {
            child: child,
            description: description,
            logger: JormungandrLogger::new(log_file_path.clone()),
        }
    }

    pub fn assert_no_erros_in_log(&self) {
        let error_lines = self.logger.get_lines_with_error();

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
