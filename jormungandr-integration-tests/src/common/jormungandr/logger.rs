extern crate serde;
extern crate serde_json;

use self::serde::{Deserialize, Serialize};
use crate::common::file_utils;
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

    pub fn get_log_content(&self) -> String {
        file_utils::read_file(&self.log_file_path)
    }

    pub fn get_lines_with_error(&self) -> impl Iterator<Item = String> + '_ {
        let lines = self.get_lines_from_log();
        lines.filter(move |x| self.is_error_line(x))
    }

    pub fn get_created_blocks_counter(&self) -> usize {
        self.get_log_entries()
            .filter(|x| {
                x.msg == "block added successfully to Node's blockchain" && x.task == "block"
            })
            .count()
    }

    fn is_error_line(&self, line: &String) -> bool {
        let entry = self.parse_line_as_entry(&line);
        entry.level == "ERROR"
    }

    fn parse_line_as_entry(&self, line: &String) -> LogEntry {
        let entry: LogEntry = serde_json::from_str(&line).expect(&format!(
            "Cannot parse log line into json '{}'. Please ensure json logger is used for node. Full log content: {}",
            &line,
            self.get_log_content()
        ));
        entry
    }

    pub fn get_lines_from_log(&self) -> impl Iterator<Item = String> {
        let file = File::open(self.log_file_path.clone()).unwrap();
        let reader = BufReader::new(file);
        reader.lines().map(|line| line.unwrap())
    }

    pub fn get_log_entries(&self) -> impl Iterator<Item = LogEntry> + '_ {
        self.get_lines_from_log()
            .map(move |x| self.parse_line_as_entry(&x))
    }

    pub fn contains_any_errors(&self) -> bool {
        self.get_lines_with_error().next().is_some()
    }

    pub fn print_logs_if_contain_error(&self) {
        if self.contains_any_errors() {
            println!(
                "Error lines: {:?}",
                self.get_lines_with_error().collect::<Vec<String>>()
            );
        }
    }
}
