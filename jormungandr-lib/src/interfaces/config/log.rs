use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log(pub LogEntry);

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
    File(PathBuf),
}

impl Log {
    pub fn file_path(&self) -> Option<&Path> {
        match &self.0.output {
            LogOutput::File(path) => Some(path.as_path()),
            _ => None,
        }
    }
}
