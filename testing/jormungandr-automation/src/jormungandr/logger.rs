use crate::testing::{collector::OutputCollector, Timestamp};
use chain_core::property::FromStr;
use chain_impl_mockchain::{block, key::Hash};
use jormungandr_lib::{interfaces::BlockDate, time::SystemTime};
use serde::{Deserialize, Serialize};
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    convert::AsRef,
    fmt,
    io::Read,
    ops::Index,
};
use strum::AsRefStr;

// TODO: we use a RefCell because it would be very labor intensive to change
// the rest of the testing framework to take a mutable reference to the logger
pub struct JormungandrLogger {
    collected_logs: RefCell<Vec<LogEntry>>,
    logs_collector: RefCell<OutputCollector>,
    panics_collector: RefCell<OutputCollector>,
    collected_panics: RefCell<Vec<String>>,
}

// The name is used to serialize/deserialize
#[allow(clippy::upper_case_acronyms)]
#[derive(AsRefStr, Serialize, Deserialize, Eq, PartialEq, Debug, Clone, PartialOrd)]
pub enum Level {
    #[serde(alias = "TRCE")]
    TRACE,
    #[serde(alias = "DEBG")]
    DEBUG,
    INFO,
    WARN,
    #[serde(alias = "ERRO")]
    ERROR,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::INFO
    }
}

impl std::str::FromStr for Level {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "trace" => Self::TRACE,
            "debug" => Self::DEBUG,
            "info" => Self::INFO,
            "warn" => Self::WARN,
            "error" => Self::ERROR,
            _ => return Err(format!("'{}' is not a valid log level", s)),
        })
    }
}

const SUCCESFULLY_CREATED_BLOCK_MSG: &str = "block from leader event successfully stored";
type RawFields = HashMap<String, String>;

// TODO: convert strings to enums for level/task/
// TODO: convert ts to DateTime
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogEntry {
    pub level: Level,
    #[serde(alias = "timestamp")]
    pub ts: String,
    pub fields: RawFields,
    pub target: String,
    pub span: Option<RawFields>,
    pub spans: Option<Vec<RawFields>>,
}

impl fmt::Display for LogEntry {
    // Similar to tracing_subscriber format::Full (default)
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn flatten_fields(fields: &RawFields, filter: &str) -> String {
            fields
                .iter()
                .filter(|(k, _)| k.as_str() != filter)
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(",")
        }

        write!(f, "{} {} ", self.ts, self.level.as_ref())?;
        if let Some(spans) = &self.spans {
            for span in spans {
                let span_name = span.get("name").cloned().unwrap_or_default();
                write!(f, "{}{{{}}}: ", span_name, flatten_fields(span, "name"))?;
            }
        }
        write!(f, " {}: ", self.target)?;
        let fields = flatten_fields(&self.fields, LogEntry::MESSAGE);
        write!(f, "{} {}", self.message(), fields)?;
        Ok(())
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
        macro_rules! insert_if_some {
            ($container:expr, $value:expr) => {
                if let Some(v) = $value {
                    $container.insert(stringify!($value).to_string(), v);
                }
            };
        }

        let LogEntryLegacy {
            msg,
            task,
            hash,
            reason,
            error,
            block_date,
            peer_addr,
            level,
            ts,
        } = log_entry;
        let mut fields = HashMap::new();

        fields.insert(LogEntry::MESSAGE.to_string(), msg);
        insert_if_some!(fields, task);
        insert_if_some!(fields, hash);
        insert_if_some!(fields, reason);
        insert_if_some!(fields, error);
        insert_if_some!(fields, block_date);
        insert_if_some!(fields, peer_addr);

        Self {
            level,
            ts,
            fields,
            target: String::new(),
            span: None,
            spans: None,
        }
    }
}

impl LogEntry {
    // this is the name of the field used by tracing to print messages
    const MESSAGE: &'static str = "message";

    pub fn reason_contains(&self, reason_part: &str) -> bool {
        match &self.fields.get("reason") {
            Some(reason) => reason.contains(reason_part),
            None => false,
        }
    }

    pub fn error_contains(&self, error_part: &str) -> bool {
        match &self.fields.get("error") {
            Some(error) => error.contains(error_part),
            None => false,
        }
    }

    pub fn block_date(&self) -> Option<BlockDate> {
        self.fields
            .get("block_date")
            .map(|block| block::BlockDate::from_str(block).unwrap().into())
    }

    pub fn is_later_than(&self, reference_time: &SystemTime) -> bool {
        let entry_system_time = SystemTime::from_str(&self.ts).unwrap();
        entry_system_time.duration_since(*reference_time).is_ok()
    }

    pub fn message(&self) -> String {
        self.fields.get(Self::MESSAGE).cloned().unwrap_or_default()
    }
}

impl From<LogEntry> for Timestamp {
    fn from(log_entry: LogEntry) -> Timestamp {
        log_entry.ts.parse().unwrap()
    }
}

impl JormungandrLogger {
    pub fn new<R1, R2>(logs_source: R1, panics_source: R2) -> Self
    where
        R1: Read + Send + 'static,
        R2: Read + Send + 'static,
    {
        JormungandrLogger {
            logs_collector: RefCell::new(OutputCollector::new(logs_source)),
            panics_collector: RefCell::new(OutputCollector::new(panics_source)),
            collected_logs: RefCell::new(Vec::new()),
            collected_panics: RefCell::new(Vec::new()),
        }
    }

    fn collect_available_input(&self) {
        let collected = &mut self.collected_logs.borrow_mut();
        for line in self.logs_collector.borrow_mut().take_available_input() {
            let entry = Self::try_parse_line_as_entry(&line).unwrap();
            // Filter out logs produced by other libraries
            if entry.target.starts_with("jormungandr") {
                collected.push(entry);
            }
        }

        self.collected_panics
            .borrow_mut()
            .extend(self.panics_collector.borrow_mut().take_available_input());
    }

    pub fn get_error_indicators() -> Vec<&'static str> {
        vec!["panicked"]
    }

    fn entries(&self) -> Ref<Vec<LogEntry>> {
        self.collect_available_input();
        self.collected_logs.borrow()
    }

    fn panic_entries(&self) -> Ref<Vec<String>> {
        self.collect_available_input();
        self.collected_panics.borrow()
    }

    pub fn get_log_content(&self) -> String {
        self.entries()
            .iter()
            .map(LogEntry::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn get_panic_content(&self) -> String {
        self.panic_entries().join("\n")
    }

    pub fn get_lines_as_string(&self) -> Vec<String> {
        self.entries().iter().map(|x| x.to_string()).collect()
    }

    pub fn get_lines(&self) -> Vec<LogEntry> {
        self.entries().clone()
    }

    pub fn get_panic_lines(&self) -> Vec<String> {
        self.panic_entries().clone()
    }

    pub fn get_log_lines_with_level(&self, level: Level) -> impl Iterator<Item = LogEntry> {
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
                    .any(|indicator| entry.message().contains(indicator))
        }) || !self.panic_entries().is_empty()
    }

    pub fn last_validated_block_date(&self) -> Option<BlockDate> {
        self.entries()
            .iter()
            .filter(|x| x.message().contains("validated block"))
            .map(|x| x.block_date())
            .last()
            .unwrap_or(None)
    }

    pub fn contains_any_of(&self, messages: &[&str]) -> bool {
        self.entries()
            .iter()
            .any(|line| messages.iter().any(|x| line.message().contains(x)))
    }

    pub fn get_created_blocks_hashes(&self) -> Vec<Hash> {
        self.filter_entries_with_block_creation()
            .map(|item| Hash::from_str(item.span.unwrap().get("hash").unwrap()).unwrap())
            .collect()
    }

    pub fn get_created_blocks_hashes_after(&self, reference_time: SystemTime) -> Vec<Hash> {
        self.filter_entries_with_block_creation()
            .filter(|item| item.is_later_than(&reference_time))
            .map(|item| Hash::from_str(item.span.unwrap().get("hash").unwrap()).unwrap())
            .collect()
    }

    pub fn get_created_blocks_counter(&self) -> usize {
        self.filter_entries_with_block_creation().count()
    }

    fn filter_entries_with_block_creation(&self) -> impl Iterator<Item = LogEntry> {
        self.entries().clone().into_iter().filter(move |x| {
            x.message() == SUCCESFULLY_CREATED_BLOCK_MSG
                && x.span.as_ref().and_then(|span| span.get("hash")).is_some()
        })
    }

    pub fn assert_no_errors(&self, message: &str) {
        let error_lines = self
            .get_log_lines_with_level(Level::ERROR)
            .collect::<Vec<_>>();

        let panics = self.panic_entries();

        assert_eq!(
            panics.len(),
            0,
            "{} there are some panics: {:?}",
            message,
            panics,
        );

        assert_eq!(
            error_lines.len(),
            0,
            "{} there are some errors in log: {:?}",
            message,
            error_lines,
        );
    }

    fn try_parse_line_as_entry(line: &str) -> Result<LogEntry, serde_json::Error> {
        use serde_json::Value;
        // try legacy log first
        let legacy_entry: Result<LogEntryLegacy, _> = serde_json::from_str(line);
        if let Ok(result) = legacy_entry {
            return Ok(result.into());
        }

        // Fields could be of various types, since we do not need to modify or
        // interact with them, it is easier to just map them to strings
        fn stringify_map<K: Index<Q, Output = Value>, Q>(container: &K, field: Q) -> Value {
            container
                .index(field)
                .as_object()
                .expect("not an object")
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        if v.is_string() {
                            v.to_owned()
                        } else {
                            serde_json::Value::String(v.to_string())
                        },
                    )
                })
                .collect()
        }

        let mut value: Value = serde_json::from_str(line).unwrap();
        value["fields"] = stringify_map(&value, "fields");
        if value.get("span").is_some() {
            value["span"] = stringify_map(&value, "span");
        }
        let spans = value.get_mut("spans").and_then(|x| x.as_array_mut());
        if let Some(spans) = spans {
            for i in 0..spans.len() {
                spans[i] = stringify_map(&*spans, i);
            }
        }
        serde_json::from_value(value)
    }
}
