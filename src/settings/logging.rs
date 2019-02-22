use slog::Drain;
use std::str::FromStr;

use crate::log_wrapper;

#[derive(Debug)]
pub struct LogSettings {
    pub verbosity: slog::Level,
    pub format: LogFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// Format of the logger.
pub enum LogFormat {
    Plain,
    Json,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cleared = s.trim().to_lowercase();
        if cleared == "plain" {
            Ok(LogFormat::Plain)
        } else if cleared == "json" {
            Ok(LogFormat::Json)
        } else {
            let mut msg = "unknown format ".to_string();
            msg.push_str(&cleared);
            Err(msg)
        }
    }
}

impl LogSettings {
    /// Configure logger subsystem based on the options that were passed.
    pub fn apply(&self) {
        let log = match self.format {
            // XXX: Some code duplication here as rust compiler dislike
            // that branches return Drain's of different type.
            LogFormat::Plain => {
                let decorator = slog_term::TermDecorator::new().build();
                let drain = slog_term::FullFormat::new(decorator).build().fuse();
                let drain = slog::LevelFilter::new(drain, self.verbosity).fuse();
                let drain = slog_async::Async::new(drain).build().fuse();
                slog::Logger::root(drain, o!())
            }
            LogFormat::Json => {
                let drain = slog_json::Json::default(std::io::stderr()).fuse();
                let drain = slog::LevelFilter::new(drain, self.verbosity).fuse();
                let drain = slog_async::Async::new(drain).build().fuse();
                slog::Logger::root(drain, o!())
            }
        };
        log_wrapper::logger::set_global_logger(log);
    }
}
