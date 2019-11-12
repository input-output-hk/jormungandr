use crate::log::AsyncableDrain;
use slog::{Drain, FilterLevel, Logger};
use slog_async::Async;
#[cfg(feature = "gelf")]
use slog_gelf::Gelf;
#[cfg(feature = "systemd")]
use slog_journald::JournaldDrain;
#[cfg(unix)]
use slog_syslog::Facility;
use slog_term::{PlainDecorator, TermDecorator};
use std::error;
use std::fmt::{self, Display};
use std::fs;
use std::io;
use std::str::FromStr;

#[derive(Debug)]
pub struct LogSettings {
    pub level: FilterLevel,
    pub format: LogFormat,
    pub output: LogOutput,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// Format of the logger.
pub enum LogFormat {
    Plain,
    Json,
}

impl Display for LogFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            LogFormat::Plain => "plain",
            LogFormat::Json => "json",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// Output of the logger.
pub enum LogOutput {
    Stdout,
    Stderr,
    #[cfg(unix)]
    Syslog,
    #[cfg(feature = "systemd")]
    Journald,
    #[cfg(feature = "gelf")]
    Gelf {
        backend: String,
        log_id: String,
    },
    File(String),
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
        match s.trim().to_lowercase().as_str() {
            "stdout" => Ok(LogOutput::Stdout),
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
        let filter_level = self.level;
        let drain = self
            .output
            .to_logger(&self.format)?
            .filter(move |record| filter_level.accepts(record.level()))
            .fuse();
        Ok(slog::Logger::root(drain, o!()))
    }
}

impl LogOutput {
    fn to_logger(&self, format: &LogFormat) -> Result<Async, Error> {
        match self {
            LogOutput::Stdout => Ok(format.decorate_stdout()),
            LogOutput::Stderr => Ok(format.decorate_stderr()),
            #[cfg(unix)]
            LogOutput::Syslog => {
                format.require_plain()?;
                match slog_syslog::unix_3164(Facility::LOG_USER) {
                    Ok(drain) => Ok(drain.into_async()),
                    Err(e) => Err(Error::SyslogAccessFailed(e)),
                }
            }
            #[cfg(feature = "systemd")]
            LogOutput::Journald => {
                format.require_plain()?;
                Ok(JournaldDrain.into_async())
            }
            #[cfg(feature = "gelf")]
            LogOutput::Gelf {
                backend: graylog_host_port,
                log_id: graylog_source,
            } => {
                // Both currently recognized formats can be understood to apply:
                // GELF formats payloads in JSON so 'json' is redundant,
                // and plain messages are worked into JSON just the same.
                // Match them irrefutably so that any new format will need to
                // be addressed here when added.
                match format {
                    LogFormat::Plain | LogFormat::Json => {}
                };
                let gelf_drain = Gelf::new(graylog_source, graylog_host_port)
                    .map_err(Error::GelfConnectionFailed)?;
                // We also log to stderr otherwise users see no logs.
                // TODO: remove when multiple output is properly supported.
                let stderr_drain = format.decorate_stderr();
                Ok(slog::Duplicate(gelf_drain, stderr_drain).into_async())
            }
            LogOutput::File(path) => {
                let file = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(path)
                    .map_err(Error::FileError)?;
                Ok(format.decorate_writer(file))
            }
        }
    }
}

fn term_drain_with_decorator<D>(d: D) -> slog_term::FullFormat<D>
where
    D: slog_term::Decorator + Send + 'static,
{
    slog_term::FullFormat::new(d).build()
}

impl LogFormat {
    fn require_plain(&self) -> Result<(), Error> {
        match self {
            LogFormat::Plain => Ok(()),
            _ => Err(Error::PlainFormatRequired { specified: *self }),
        }
    }

    fn decorate_stdout(&self) -> Async {
        match self {
            LogFormat::Plain => {
                term_drain_with_decorator(TermDecorator::new().stdout().build()).into_async()
            }
            LogFormat::Json => slog_json::Json::default(io::stdout()).into_async(),
        }
    }

    fn decorate_stderr(&self) -> Async {
        match self {
            LogFormat::Plain => {
                term_drain_with_decorator(TermDecorator::new().stderr().build()).into_async()
            }
            LogFormat::Json => slog_json::Json::default(io::stderr()).into_async(),
        }
    }

    fn decorate_writer<T: io::Write + Send + 'static>(&self, w: T) -> Async {
        match self {
            LogFormat::Plain => term_drain_with_decorator(PlainDecorator::new(w)).into_async(),
            LogFormat::Json => slog_json::Json::default(w).into_async(),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    PlainFormatRequired {
        specified: LogFormat,
    },
    #[cfg(unix)]
    SyslogAccessFailed(io::Error),
    #[cfg(feature = "gelf")]
    GelfConnectionFailed(io::Error),
    FileError(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::PlainFormatRequired { specified } => write!(
                f,
                "log format `{}` is not supported for this output",
                specified
            ),
            #[cfg(unix)]
            Error::SyslogAccessFailed(_) => write!(f, "syslog access failed"),
            #[cfg(feature = "gelf")]
            Error::GelfConnectionFailed(_) => write!(f, "GELF connection failed"),
            Error::FileError(e) => write!(f, "failed to open the log file: {}", e),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::PlainFormatRequired { .. } => None,
            #[cfg(unix)]
            Error::SyslogAccessFailed(err) => Some(err),
            #[cfg(feature = "gelf")]
            Error::GelfConnectionFailed(err) => Some(err),
            Error::FileError(err) => Some(err),
        }
    }
}
