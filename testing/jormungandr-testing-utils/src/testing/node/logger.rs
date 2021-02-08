use chain_core::property::FromStr;
use chain_impl_mockchain::{block, key::Hash};
use jortestkit::file as file_utils;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use thiserror::Error;

use crate::testing::Timestamp;
use jormungandr_lib::{interfaces::BlockDate, time::SystemTime};
use std::collections::HashMap;

#[derive(Debug, Error)]
pub enum LoggerError {
    #[error("{log_file}")]
    LogFileDoesNotExist { log_file: String },
}

#[derive(Debug, Clone)]
pub struct JormungandrLogger {
    pub log_file_path: PathBuf,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub enum Level {
    WARN,
    INFO,
    ERROR,
    TRACE,
    DEBUG,
}

const SUCCESFULLY_CREATED_BLOCK_MSG: &str = "block from leader event successfully stored";

#[derive(Serialize, Deserialize, Debug)]
pub struct Fields {
    #[serde(alias = "message")]
    pub msg: String,
    #[serde(alias = "kind")]
    pub task: Option<String>,
    pub hash: Option<String>,
    pub reason: Option<String>,
    pub error: Option<String>,
    pub block_date: Option<String>,
    pub peer_addr: Option<String>,
}

// TODO: convert strings to enums for level/task/
// TODO: convert ts to DateTime
#[derive(Serialize, Deserialize, Debug)]
pub struct LogEntry {
    pub level: Level,
    #[serde(alias = "timestamp")]
    pub ts: String,
    pub fields: Fields,
}

impl LogEntry {
    pub fn reason_contains(&self, reason_part: &str) -> bool {
        match &self.fields.reason {
            Some(reason) => reason.contains(reason_part),
            None => false,
        }
    }

    pub fn error_contains(&self, error_part: &str) -> bool {
        match &self.fields.error {
            Some(error) => error.contains(error_part),
            None => false,
        }
    }

    pub fn block_date(&self) -> Option<BlockDate> {
        self.fields
            .block_date
            .clone()
            .map(|block| block::BlockDate::from_str(&block).unwrap().into())
    }

    pub fn is_later_than(&self, reference_time: &SystemTime) -> bool {
        let entry_system_time = SystemTime::from_str(&self.ts).unwrap();
        entry_system_time.duration_since(*reference_time).is_ok()
    }
}

impl Into<Timestamp> for LogEntry {
    fn into(self) -> Timestamp {
        self.ts.parse().unwrap()
    }
}

impl JormungandrLogger {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        JormungandrLogger {
            log_file_path: path.into(),
        }
    }

    pub fn get_error_indicators() -> Vec<&'static str> {
        vec!["panicked"]
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
        let panic_in_logs_found = Self::get_error_indicators()
            .iter()
            .any(|x| self.get_log_content().contains(x));

        Ok(panic_in_logs_found || self.get_lines_with_error().count() > 0)
    }

    pub fn last_validated_block_date(&self) -> Option<BlockDate> {
        self.get_log_entries()
            .filter(|x| x.fields.msg.contains("validated block"))
            .map(|x| x.block_date())
            .last()
            .unwrap_or(None)
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
        Ok(self
            .get_log_entries()
            .any(|x| x.fields.msg.contains(message)))
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
            .map(|item| Hash::from_str(&item.fields.hash.unwrap()).unwrap())
            .collect()
    }

    pub fn get_created_blocks_hashes_after(&self, reference_time: SystemTime) -> Vec<Hash> {
        self.filter_entries_with_block_creation()
            .filter(|item| item.is_later_than(&reference_time))
            .map(|item| Hash::from_str(&item.fields.hash.unwrap()).unwrap())
            .collect()
    }

    pub fn get_created_blocks_counter(&self) -> usize {
        self.filter_entries_with_block_creation().count()
    }

    fn filter_entries_with_block_creation(&self) -> impl Iterator<Item = LogEntry> + '_ {
        let expected_task = Some("block".to_string());
        self.get_log_entries().filter(move |x| {
            x.fields.msg == SUCCESFULLY_CREATED_BLOCK_MSG
                && x.fields.task == expected_task
                && x.fields.hash.is_some()
        })
    }

    fn is_error_line(&self, line: &str) -> bool {
        match self.try_parse_line_as_entry(line) {
            Ok(entry) => entry.level == Level::ERROR,
            Err(_) => false,
        }
    }

    fn is_warn_line(&self, line: &str) -> bool {
        match self.try_parse_line_as_entry(&line) {
            Ok(entry) => entry.level == Level::WARN,
            Err(_) => false,
        }
    }

    fn is_error_line_or_invalid(&self, line: &str) -> bool {
        match self.try_parse_line_as_entry(&line) {
            Ok(entry) => entry.level == Level::ERROR,
            Err(_) => true,
        }
    }

    fn try_parse_line_as_entry(&self, line: &str) -> Result<LogEntry, impl std::error::Error> {
        let mut jsonize_entry: serde_json::Value = serde_json::from_str(&line)?;
        let mut aggreagated: HashMap<String, serde_json::Value> = HashMap::new();
        if let Some(fields) = jsonize_entry.get_mut("fields") {
            // main span "fields" is ensured be an object, it is safe to unwrap here
            for (k, v) in fields.take().as_object().unwrap().iter() {
                aggreagated.insert(k.clone(), v.clone());
            }
        }
        if let Some(main_span) = jsonize_entry.get("span") {
            // main span "span" is ensured be an object, it is safe to unwrap here
            for (k, v) in main_span.as_object().unwrap().iter() {
                aggreagated.insert(k.clone(), v.clone());
            }
        }
        if let Some(spans) = jsonize_entry.get("spans") {
            // spans is ensured to be an array, so it should be safe to unwrap here
            for s in spans.as_array().unwrap() {
                // same here, inner spans are represented as objects
                let span_values = s.as_object().unwrap();
                for (k, v) in span_values.iter() {
                    aggreagated.insert(k.clone(), v.clone());
                }
            }
        }
        *jsonize_entry.get_mut("fields").unwrap() = serde_json::to_value(aggreagated)?;
        serde_json::from_value(jsonize_entry)
    }

    pub fn get_lines_from_log(&self) -> impl Iterator<Item = String> {
        let file = File::open(self.log_file_path.clone())
            .unwrap_or_else(|_| panic!("cannot find log file: {:?}", &self.log_file_path));
        let reader = BufReader::new(file);
        reader.lines().map(|line| line.unwrap())
    }

    pub fn get_log_entries(&self) -> impl Iterator<Item = LogEntry> + '_ {
        self.get_lines_from_log()
            .map(move |x| self.try_parse_line_as_entry(&x))
            .filter_map(Result::ok)
    }

    fn verify_file_exists(&self) -> Result<(), LoggerError> {
        if self.log_file_path.exists() {
            Ok(())
        } else {
            Err(LoggerError::LogFileDoesNotExist {
                log_file: self.log_file_path.to_str().unwrap().to_string(),
            })
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
            .filter(|x| x.fields.msg.contains(message))
            .count()
            == count)
    }

    pub fn print_error_and_invalid_logs(&self) {
        let error_lines: Vec<_> = self
            .get_lines_with_error_and_invalid()
            .map(|x| self.remove_white_space(&x))
            .collect();

        if !error_lines.is_empty() {
            println!("Error lines:");
            for line in error_lines {
                println!("{}", line);
            }
        }
    }

    fn remove_white_space(&self, input: &str) -> String {
        input.split_whitespace().collect::<String>()
    }

    pub fn print_error_or_warn_lines(&self) {
        let error_lines: Vec<_> = self.get_lines_with_error_and_warn().collect();
        if !error_lines.is_empty() {
            println!("Error/Warn lines: {:?}", error_lines);
        }
    }

    pub fn assert_no_errors(&self, message: &str) {
        let error_lines = self.get_lines_with_error().collect::<Vec<String>>();

        assert_eq!(
            error_lines.len(),
            0,
            "{} there are some errors in log ({:?}): {:?}",
            message,
            self.log_file_path,
            error_lines,
        );
    }
}
