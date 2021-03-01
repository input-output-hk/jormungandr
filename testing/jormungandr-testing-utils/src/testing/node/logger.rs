use crate::testing::Timestamp;
use chain_core::property::FromStr;
use chain_impl_mockchain::{block, key::Hash};
use jormungandr_lib::{interfaces::BlockDate, time::SystemTime};
use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::io::BufRead;
use std::process::ChildStdout;
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

// TODO: we use a RefCell because it would be very labor intensive to change
// the rest of the testing framework to take a mutable reference to the logger
pub struct JormungandrLogger {
    collected: RefCell<Vec<LogEntry>>,
    rx: Receiver<(Instant, String)>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone, PartialOrd)]
pub enum Level {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

const SUCCESFULLY_CREATED_BLOCK_MSG: &str = "block from leader event successfully stored";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Fields {
    #[serde(alias = "message", default = "String::new")]
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogEntry {
    pub level: Level,
    #[serde(alias = "timestamp")]
    pub ts: String,
    pub fields: Fields,
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(&self).unwrap())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LogEntryLegacy {
    pub level: Level,
    pub ts: String,
    pub msg: String,
    pub task: Option<String>,
    pub hash: Option<String>,
    pub reason: Option<String>,
    pub error: Option<String>,
    pub block_date: Option<String>,
    pub peer_addr: Option<String>,
}

impl From<LogEntryLegacy> for LogEntry {
    fn from(log_entry: LogEntryLegacy) -> Self {
        Self {
            level: log_entry.level,
            ts: log_entry.ts,
            fields: Fields {
                msg: log_entry.msg,
                task: log_entry.task,
                hash: log_entry.hash,
                reason: log_entry.reason,
                error: log_entry.error,
                block_date: log_entry.block_date,
                peer_addr: log_entry.peer_addr,
            },
        }
    }
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
    pub fn new(source: ChildStdout) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let lines = std::io::BufReader::new(source).lines();
            for line in lines {
                tx.send((Instant::now(), line.unwrap())).unwrap();
            }
        });
        JormungandrLogger {
            rx,
            collected: RefCell::new(Vec::new()),
        }
    }

    fn collect_available_input(&self) {
        let collected = &mut self.collected.borrow_mut();
        let now = Instant::now();
        while let Ok((time, line)) = self.rx.try_recv() {
            // we are reading from logs produce from the node, if they are not valid there is something wrong
            collected.push(Self::try_parse_line_as_entry(&line).unwrap());
            // Stop reading if the are more recent messages available, otherwise
            // we risk that a very active process could result in endless collection
            // of its output
            if time > now {
                break;
            }
        }
    }

    pub fn get_error_indicators() -> Vec<&'static str> {
        vec!["panicked"]
    }

    fn entries(&self) -> Ref<Vec<LogEntry>> {
        self.collect_available_input();
        self.collected.borrow()
    }

    pub fn get_log_content(&self) -> Vec<String> {
        self.entries().iter().map(LogEntry::to_string).collect()
    }

    pub fn get_lines_with_level(&self, level: Level) -> impl Iterator<Item = LogEntry> {
        self.entries()
            .clone()
            .into_iter()
            .filter(move |x| x.level == level)
    }

    pub fn contains_error(&self) -> bool {
        self.entries().iter().any(|entry| {
            entry.level == Level::ERROR
                || Self::get_error_indicators()
                    .iter()
                    .any(|indicator| entry.fields.msg.contains(indicator))
        })
    }

    pub fn last_validated_block_date(&self) -> Option<BlockDate> {
        self.entries()
            .iter()
            .filter(|x| x.fields.msg.contains("validated block"))
            .map(|x| x.block_date())
            .last()
            .unwrap_or(None)
    }

    pub fn contains_any_of(&self, messages: &[&str]) -> bool {
        self.entries()
            .iter()
            .any(|line| messages.iter().any(|x| line.contains(x)))
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

    fn filter_entries_with_block_creation(&self) -> impl Iterator<Item = LogEntry> {
        let expected_task = Some("block".to_string());
        self.entries().clone().into_iter().filter(move |x| {
            x.fields.msg == SUCCESFULLY_CREATED_BLOCK_MSG
                && x.fields.task == expected_task
                && x.fields.hash.is_some()
        })
    }

    pub fn assert_no_errors(&mut self, message: &str) {
        let error_lines = self.get_lines_with_level(Level::ERROR).collect::<Vec<_>>();

        assert_eq!(
            error_lines.len(),
            0,
            "{} there are some errors in log: {:?}",
            message,
            error_lines,
        );
    }

    fn try_parse_line_as_entry(line: &str) -> Result<LogEntry, serde_json::Error> {
        // try legacy log first
        let legacy_entry: Result<LogEntryLegacy, _> = serde_json::from_str(line);
        if let Ok(result) = legacy_entry {
            return Ok(result.into());
        }
        // parse and aggregate spans fields
        let mut jsonize_entry: serde_json::Value = serde_json::from_str(&line)?;
        let mut aggreagated: HashMap<String, serde_json::Value> = HashMap::new();
        if let Some(fields) = jsonize_entry.get("fields") {
            // main span "fields" is ensured be an object, it is safe to unwrap here
            for (k, v) in fields.as_object().unwrap().iter() {
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
        *(jsonize_entry
            .get_mut("fields")
            .ok_or_else(|| serde_json::error::Error::custom("Could not get mutable `fields`"))?) =
            serde_json::to_value(aggreagated)?;
        serde_json::from_value(jsonize_entry)
    }
}
