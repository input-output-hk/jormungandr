extern crate serde;
extern crate serde_json;

use self::serde::{Deserialize, Serialize};
use self::serde_json::Result;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug)]
pub struct JormungandrLogger {
    pub log_file_path: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct LogEntry {
    msg: String,
    level: String,
    ts: String,
    task: String,
}

impl JormungandrLogger {
    pub fn new(log_file_path: PathBuf) -> Self {
        JormungandrLogger { log_file_path }
    }

    pub fn get_lines_with_error(&self) -> impl Iterator<Item = String> {
        let lines = self.get_lines_from_log();
        lines.filter(JormungandrLogger::is_error_line)
    }

    fn is_error_line(line: &String) -> bool {
        let entry: LogEntry = serde_json::from_str(&line).expect(&format!(
            "Cannot parse log line into json '{}'. Please ensure json logger is used for node",
            &line
        ));
        entry.level == "ERROR"
    }

    fn get_lines_from_log(&self) -> impl Iterator<Item = String> {
        let file = File::open(self.log_file_path.clone()).unwrap();
        let mut data: Vec<String> = Vec::new();
        let reader = BufReader::new(file);

        for (_index, line) in reader.lines().enumerate() {
            data.push(line.unwrap());
        }
        data.into_iter()
    }
}
