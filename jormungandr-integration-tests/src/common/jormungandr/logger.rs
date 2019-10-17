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
    task: Option<String>,
}

impl JormungandrLogger {
    pub fn new(log_file_path: PathBuf) -> Self {
        JormungandrLogger { log_file_path }
    }

    pub fn get_error_indicators() -> Vec<&'static str> {
        vec!["panicked", "|->"]
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

    pub fn contains_error(&self) -> bool {
        Self::get_error_indicators()
            .iter()
            .any(|x| self.get_log_content().contains(x))
    }

    pub fn print_raw_log(&self) {
        println!("{}", self.get_log_content());
    }

    pub fn contains_message(&self, message: &str) -> bool {
        self.get_log_entries().any(|x| x.msg.contains(message))
    }

    pub fn get_created_blocks_counter(&self) -> usize {
        let expected_task = Some("block".to_string());
        self.get_log_entries()
            .filter(|x| {
                x.msg == "block added successfully to Node's blockchain" && x.task == expected_task
            })
            .count()
    }

    fn is_error_line(&self, line: &String) -> bool {
        match self.try_parse_line_as_entry(&line) {
            Ok(entry) => entry.level == "ERROR",
            Err(_) => false,
        }
    }

    fn is_error_line_or_invalid(&self, line: &String) -> bool {
        match self.try_parse_line_as_entry(&line) {
            Ok(entry) => entry.level == "ERROR",
            Err(_) => true,
        }
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
            .map(move |x| self.try_parse_line_as_entry(&x))
            .filter_map(Result::ok)
    }

    pub fn print_error_and_invalid_logs(&self) {
        let error_lines: Vec<_> = self.get_lines_with_error_and_invalid().collect();
        if !error_lines.is_empty() {
            println!("Error lines: {:?}", error_lines);
        }
    }
}
