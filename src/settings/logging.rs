use slog::{Drain, Logger};
use std::str::FromStr;

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
    pub fn to_logger(&self) -> Logger {
        let drain = match self.format {
            LogFormat::Plain => {
                let decorator = slog_term::TermDecorator::new().build();
                let drain = slog_term::FullFormat::new(decorator).build().fuse();
                slog_async::Async::new(drain).build().fuse()
            }
            LogFormat::Json => {
                let drain = slog_json::Json::default(std::io::stderr()).fuse();
                slog_async::Async::new(drain).build().fuse()
            }
        };
        let drain = slog::LevelFilter::new(drain, self.verbosity).fuse();
        slog::Logger::root(drain, o!())
    }
}
