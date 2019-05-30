use slog::{Drain, Logger};
use std::str::FromStr;

#[derive(Debug)]
pub struct LogSettings {
    pub verbosity: slog::Level,
    pub format: LogFormat,
    pub output: LogOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// Format of the logger.
pub enum LogFormat {
    Plain,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// Output of the logger.
pub enum LogOutput {
    Stderr,
    #[cfg(unix)]
    Syslog,
    #[cfg(feature = "systemd")]
    Journald,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.trim().to_lowercase() {
            "plain" => Ok(LogFormat::Plain),
            "json" => Ok(LogFormat::Json),
            other => Err(format!("unknown log format '{}'", other)),
        }
    }
}

impl FromStr for LogOutput {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.trim().to_lowercase() {
            "stderr" => Ok(LogOutput::Stderr),
            #[cfg(unix)]
            "syslog" => Ok(LogOutput::Syslog),
            #[cfg(feature = "systemd")]
            "journald" => Ok(LogOutput::Journald),
            other => Err(format!("unknown log output '{}'", other)),
        }
    }
}

impl LogSettings {
    pub fn to_logger(&self) -> Result<Logger, Error> {
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
        Ok(slog::Logger::root(drain, o!()))
    }
}

custom_error! {pub Error
    ImpossibleError {} = @{{ unreachable!(); "" }} // Custom_error requires at least 1 variant
}
