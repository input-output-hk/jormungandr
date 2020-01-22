use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log(pub Vec<LogEntry>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub format: String,
    pub level: String,
    pub output: LogOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogOutput {
    Stdout,
    Stderr,
    File(String),
}

impl Log {
    pub fn log_file(&self) -> Option<PathBuf> {
        match self.file_logger_entry() {
            Some(log_entry) => match log_entry.output {
                LogOutput::File(file) => Some(PathBuf::from(file)),
                _ => None,
            },
            None => None,
        }
    }

    pub fn file_logger_entry(&self) -> Option<LogEntry> {
        self.0.iter().cloned().find(|x| match x.output {
            LogOutput::File(_) => true,
            _ => false,
        })
    }
}
