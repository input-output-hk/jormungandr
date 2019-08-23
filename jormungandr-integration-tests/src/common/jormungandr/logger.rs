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

    pub fn get_lines_with_error_and_invalid(&self) -> impl Iterator<Item = String> + '_ {
        let lines = self.get_lines_from_log();
        lines.filter(move |x| self.is_error_line_or_invalid(x))
    }

    pub fn get_created_blocks_counter(&self) -> usize {
        self.get_log_entries()
            .filter(|x| {
                x.msg == "block added successfully to Node's blockchain" && x.task == "block"
            })
            .count()
    }

    fn is_error_line(&self, line: &String) -> bool {
        self.parse_line_as_entry(&line).level == "ERROR"
    }

    fn is_error_line_or_invalid(&self, line: &String) -> bool {
        match self.try_parse_line_as_entry(&line) {
            Ok(entry) => entry.level == "ERROR",
            Err(_) => true,
        }
    }

    fn parse_line_as_entry(&self, line: &String) -> LogEntry {
        self.try_parse_line_as_entry(line).unwrap_or_else(|error| panic!(
            "Cannot parse log line into json '{}': {}. Please ensure json logger is used for node. Full log content: {}",
            &line,
            error,
            self.get_log_content()
        ))
    }

    fn try_parse_line_as_entry(&self, line: &String) -> Result<LogEntry, impl std::error::Error> {
        serde_json::from_str(&line)
    }

    fn get_lines_from_log(&self) -> impl Iterator<Item = String> {
        let file = File::open(self.log_file_path.clone())
            .expect(&format!("cannot find log file: {:?}", &self.log_file_path));
        let reader = BufReader::new(file);
        reader.lines().map(|line| line.unwrap())
    }

    fn get_log_entries(&self) -> impl Iterator<Item = LogEntry> + '_ {
        self.get_lines_from_log()
            .map(move |x| self.parse_line_as_entry(&x))
    }

    pub fn print_error_and_invalid_logs(&self) {
        let error_lines: Vec<_> = self.get_lines_with_error_and_invalid().collect();
        if !error_lines.is_empty() {
            println!("Error lines: {:?}", error_lines);
        }
    }
}
