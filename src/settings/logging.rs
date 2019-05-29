use slog::{Drain, Logger};
use std::{io, str::FromStr};

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
    #[cfg(unix)]
    Syslog,
    #[cfg(feature = "systemd")]
    Journald,
}

impl FromStr for LogOutput {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.trim().to_lowercase() {
            "stderr" => Ok(LogOutput::Stderr),
            "stderr_json" => Ok(LogOutput::StderrJson),
            #[cfg(unix)]
            "syslog" => Ok(LogOutput::Syslog),
            #[cfg(feature = "systemd")]
            "journald" => Ok(LogOutput::Journald),
            other => Err(format!("unknown format '{}'", other)),
        }
    }
}

impl LogSettings {
    pub fn to_logger(&self) -> Result<Logger, Error> {
        let drain = match self.output {
            LogOutput::Stderr => {
                let decorator = slog_term::TermDecorator::new().build();
                let drain = slog_term::FullFormat::new(decorator).build().fuse();
                slog_async::Async::new(drain).build()
            }
            LogOutput::StderrJson => {
                let drain = slog_json::Json::default(std::io::stderr()).fuse();
                slog_async::Async::new(drain).build()
            }
            #[cfg(unix)]
            LogOutput::Syslog => {
                let drain = slog_syslog::unix_3164(slog_syslog::Facility::LOG_USER)?.fuse();
                slog_async::Async::new(drain).build()
            }
            #[cfg(feature = "systemd")]
            LogOutput::Journald => {
                let drain = slog_journald::JournaldDrain.fuse();
                slog_async::Async::new(drain).build()
            }
        };
        let drain = slog::LevelFilter::new(drain.fuse(), self.verbosity).fuse();
        Ok(slog::Logger::root(drain, o!()))
    }
}

custom_error! {pub Error
    SyslogAccessFailed { source: io::Error } = "syslog access failed",
}

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;

    #[test]
    fn stderr_smoke_test() {
        smoke_test(LogOutput::Stderr)
    }

    #[test]
    fn stderr_json_smoke_test() {
        smoke_test(LogOutput::StderrJson)
    }

    #[cfg(unix)]
    #[test]
    fn syslog_smoke_test() {
        smoke_test(LogOutput::Syslog)
    }

    #[cfg(feature = "systemd")]
    #[test]
    fn journald_smoke_test() {
        smoke_test(LogOutput::Journald)
    }

    fn smoke_test(output: LogOutput) {
        let settings = LogSettings {
            verbosity: slog::Level::Debug,
            output,
        };

        let logger = settings.to_logger().expect("Failed to create logger");
        debug!(logger, "smoke test");
    }
}
