extern crate custom_error;
extern crate regex;
extern crate serde;
extern crate serde_json;

use self::custom_error::custom_error;

use self::serde::{Deserialize, Serialize};
use crate::common::file_utils;
use chain_core::property::FromStr;
use chain_impl_mockchain::key::Hash;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

custom_error! {pub LoggerError
    LogFileDoesNotExists { log_file: String } = "{log_file}",
}

#[derive(Debug)]
pub struct JormungandrLogger {
    pub log_file_path: PathBuf,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub enum Level {
    WARN,
    INFO,
    ERRO,
}

const SUCCESFULLY_CREATED_BLOCK_MSG: &str = "block from leader event successfully stored";

// TODO: convert strings to enums for level/task/
// TODO: convert ts to DateTime
#[derive(Serialize, Deserialize)]
pub struct LogEntry {
    pub msg: String,
    pub level: Level,
    pub ts: String,
    pub task: Option<String>,
    pub hash: Option<String>,
    pub reason: Option<String>,
    pub peer_addr: Option<String>,
}

impl LogEntry {
    pub fn reason_contains(&self, reason_part: &str) -> bool {
        match &self.reason {
            Some(reason) => reason.contains(reason_part),
            None => false,
        }
    }
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

    pub fn contains_error(&self) -> Result<bool, LoggerError> {
        self.verify_file_exists()?;
        Ok(Self::get_error_indicators()
            .iter()
            .any(|x| self.get_log_content().contains(x)))
    }

    pub fn print_raw_log(&self) {
        println!("{}", self.get_log_content());
    }

    pub fn raw_log_contains_any_of(&self, messages: &[&str]) -> Result<bool, LoggerError> {
        for message in messages {
            if self.get_log_content().contains(message) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn contains_message(&self, message: &str) -> Result<bool, LoggerError> {
        self.verify_file_exists()?;
        Ok(self.get_log_entries().any(|x| x.msg.contains(message)))
    }

    pub fn get_lines_with_warn(&self) -> impl Iterator<Item = String> + '_ {
        let lines = self.get_lines_from_log();
        lines.filter(move |x| self.is_warn_line(x))
    }

    pub fn get_lines_with_error_and_warn(&self) -> impl Iterator<Item = String> + '_ {
        let lines = self.get_lines_from_log();
        lines.filter(move |x| self.is_warn_line(x) || self.is_error_line(x))
    }

    pub fn get_created_blocks_hashes(&self) -> Vec<Hash> {
        self.filter_entries_with_block_creation()
            .map(|item| Hash::from_str(&item.hash.unwrap()).unwrap())
            .collect()
    }

    pub fn get_created_blocks_counter(&self) -> usize {
        self.filter_entries_with_block_creation().count()
    }

    fn filter_entries_with_block_creation(&self) -> impl Iterator<Item = LogEntry> + '_ {
        let expected_task = Some("block".to_string());
        self.get_log_entries().filter(move |x| {
            x.msg == SUCCESFULLY_CREATED_BLOCK_MSG && x.task == expected_task && x.hash.is_some()
        })
    }

    fn is_error_line(&self, line: &String) -> bool {
        match self.try_parse_line_as_entry(&line) {
            Ok(entry) => entry.level == Level::ERRO,
            Err(_) => false,
        }
    }

    fn is_warn_line(&self, line: &String) -> bool {
        match self.try_parse_line_as_entry(&line) {
            Ok(entry) => entry.level == Level::WARN,
            Err(_) => false,
        }
    }

    fn is_error_line_or_invalid(&self, line: &String) -> bool {
        match self.try_parse_line_as_entry(&line) {
            Ok(entry) => entry.level == Level::ERRO,
            Err(_) => true,
        }
    }

    fn try_parse_line_as_entry(&self, line: &String) -> Result<LogEntry, impl std::error::Error> {
        serde_json::from_str(&line)
    }

    pub fn get_lines_from_log(&self) -> impl Iterator<Item = String> {
        let file = File::open(self.log_file_path.clone())
            .expect(&format!("cannot find log file: {:?}", &self.log_file_path));
        let reader = BufReader::new(file);
        reader.lines().map(|line| line.unwrap())
    }

    pub fn get_log_entries(&self) -> impl Iterator<Item = LogEntry> + '_ {
        self.get_lines_from_log()
            .map(move |x| self.try_parse_line_as_entry(&x))
            .filter_map(Result::ok)
    }

    fn verify_file_exists(&self) -> Result<(), LoggerError> {
        match self.log_file_path.exists() {
            true => Ok(()),
            false => Err(LoggerError::LogFileDoesNotExists {
                log_file: self.log_file_path.to_str().unwrap().to_string(),
            }),
        }
    }

    pub fn message_logged_multiple_times(
        &self,
        message: &str,
        count: usize,
    ) -> Result<bool, LoggerError> {
        self.verify_file_exists()?;

        Ok(self
            .get_log_entries()
            .filter(|x| x.msg.contains(message))
            .count()
            == count)
    }

    pub fn print_error_and_invalid_logs(&self) {
        let error_lines: Vec<_> = self.get_lines_with_error_and_invalid().collect();
        if !error_lines.is_empty() {
            println!("Error lines: {:?}", error_lines);
        }
    }

    pub fn print_error_or_warn_lines(&self) {
        let error_lines: Vec<_> = self.get_lines_with_error_and_warn().collect();
        if !error_lines.is_empty() {
            println!("Error/Warn lines: {:?}", error_lines);
        }
    }
}
