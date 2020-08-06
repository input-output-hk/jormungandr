use crate::testing::file as file_utils;
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::{fmt, fs::File, io::BufReader, path::PathBuf};
#[derive(Debug)]
pub struct MockLogger {
    pub log_file_path: PathBuf,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub enum MethodType {
    Init,
    Handshake,
    PullBlocksToTip,
    Tip,
    GetBlocks,
    GetHeaders,
    GetFragments,
    GetPeers,
    PullHeaders,
    PushHeaders,
    UploadBlocks,
    BlockSubscription,
    FragmentSubscription,
    GossipSubscription,
}

impl fmt::Display for MethodType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub enum Level {
    WARN,
    INFO,
    ERRO,
}

#[derive(Serialize, Deserialize)]
pub struct LogEntry {
    pub msg: String,
    pub level: Level,
    pub ts: String,
    pub method: MethodType,
}

impl MockLogger {
    pub fn new(log_file_path: impl Into<PathBuf>) -> Self {
        Self {
            log_file_path: log_file_path.into(),
        }
    }

    pub fn get_log_content(&self) -> String {
        file_utils::read_file(&self.log_file_path)
    }

    fn parse_line_as_entry(&self, line: &str) -> LogEntry {
        self.try_parse_line_as_entry(line).unwrap_or_else(|error| panic!(
            "Cannot parse log line into json '{}': {}. Please ensure json logger is used for node. Full log content: {}",
            &line,
            error,
            self.get_log_content()
        ))
    }

    fn try_parse_line_as_entry(&self, line: &str) -> Result<LogEntry, impl std::error::Error> {
        serde_json::from_str(&line)
    }

    pub fn get_log_entries(&self) -> impl Iterator<Item = LogEntry> + '_ {
        self.get_lines_from_log()
            .map(move |x| self.parse_line_as_entry(&x))
    }

    pub fn executed_at_least_once(&self, method: MethodType) -> bool {
        self.get_log_entries().any(|entry| entry.method == method)
    }

    fn get_lines_from_log(&self) -> impl Iterator<Item = String> {
        let file = File::open(self.log_file_path.clone())
            .unwrap_or_else(|_| panic!("cannot find log file: {:?}", &self.log_file_path));
        let reader = BufReader::new(file);
        reader.lines().map(|line| line.unwrap())
    }
}
