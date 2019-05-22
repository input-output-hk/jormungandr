use slog::{Drain, Logger};
use std::str::FromStr;

#[derive(Debug)]
pub struct LogSettings {
    pub verbosity: slog::Level,
    pub output: LogOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Format of the logger.
pub enum LogOutput {
    Stderr,
    StderrJson,
}

impl FromStr for LogOutput {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.trim().to_lowercase() {
            "stderr" => Ok(LogOutput::Stderr),
            "stderr_json" => Ok(LogOutput::StderrJson),
            other => Err(format!("unknown format '{}'", other)),
        }
    }
}

impl LogSettings {
    pub fn to_logger(&self) -> Logger {
        let drain = match self.output {
            LogOutput::Stderr => {
                let decorator = slog_term::TermDecorator::new().build();
                let drain = slog_term::FullFormat::new(decorator).build().fuse();
                slog_async::Async::new(drain).build().fuse()
            }
            LogOutput::StderrJson => {
                let drain = slog_json::Json::default(std::io::stderr()).fuse();
                slog_async::Async::new(drain).build().fuse()
            }
        };
        let drain = slog::LevelFilter::new(drain, self.verbosity).fuse();
        slog::Logger::root(drain, o!())
    }
}
