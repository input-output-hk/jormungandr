use std::error;
use std::fmt::{self, Display};
use std::fs;
use std::io;
use std::str::FromStr;

use tracing::{log::LevelFilter, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;
#[cfg(feature = "gelf")]
use tracing_gelf::Gelf;
#[cfg(feature = "systemd")]
use tracing_journald::Layer;
use tracing_subscriber::fmt::SubscriberBuilder;

pub struct LogSettings(pub Vec<LogSettingsEntry>);

#[derive(Debug)]
pub struct LogSettingsEntry {
    pub level: LevelFilter,
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
            #[cfg(feature = "systemd")]
            "journald" => Ok(LogOutput::Journald),
            other => Err(format!("unknown log output '{}'", other)),
        }
    }
}

impl LogSettings {
    pub fn init_log(&self) -> Result<Vec<WorkerGuard>, Error> {
        let mut guards = Vec::new();
        let mut subscribers: Vec<_> = Vec::new();
        for config in self.0.iter() {
            let (subscriber, guard) = config.to_subscriber();
            guards.push(guard);
            subscribers.push(subscriber);
        }
        let subscriber = subscribers
            .drain(..)
            .into_iter()
            .fold_first(|s1, s2| s1.with_subscriber(s2))
            .unwrap_or_else(|| tracing_subscriber::fmt().finish());
        tracing::subscriber::set_global_default(subscriber);
        Ok(guards)
    }
}

impl LogSettingsEntry {
    fn to_subscriber(
        &self,
    ) -> Result<
        (
            impl Subscriber,
            Option<tracing_appender::non_blocking::WorkerGuard>,
        ),
        Error,
    > {
        let Self {
            output,
            level,
            format,
        } = &self;
        let builder = format.to_subscriber_builder();
        match output {
            LogOutput::Stdout => {
                let (subscriber, guard) = tracing_appender::non_blocking(std::io::stdout());
                Ok((
                    builder
                        .with_writer(subscriber)
                        .with_max_level(level)
                        .finish(),
                    Some(guard),
                ))
            }
            LogOutput::Stderr => {
                let (subscriber, guard) = tracing_appender::non_blocking(std::io::stderr());
                Ok((
                    builder
                        .with_writer(subscriber)
                        .with_max_level(level)
                        .finish(),
                    Some(guard),
                ))
            }
            #[cfg(feature = "systemd")]
            LogOutput::Journald => {
                let layer = tracing_journald::layer()?;
                format.require_plain()?;
                Ok((builder.with_max_level(level).with_subscriber(layer), None))
            }
            #[cfg(feature = "gelf")]
            LogOutput::Gelf {
                backend: graylog_host_port,
                log_id: _graylog_source,
            } => {
                // Both currently recognized formats can be understood to apply:
                // GELF formats payloads in JSON so 'json' is redundant,
                // and plain messages are worked into JSON just the same.
                // Match them irrefutably so that any new format will need to
                // be addressed here when added.
                let address: SocketAddr = graylog_host_port.parse().unwrap();
                // TODO: maybe handle this tasks outside somehow.
                let (subscriber, task) = tracing_gelf::Logger::builder().connect_tcp(address)?;
                tokio::spawn(task);
                Ok((subscriber, None))
            }
            LogOutput::File(path) => {
                let file = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(path)
                    .map_err(Error::FileError)?;
                let (subscriber, guard) = tracing_appender::non_blocking(file);
                Ok((builder.with_writer(subscriber).finish(), Some(guard)))
            }
        }
    }
}

impl LogFormat {
    #[allow(dead_code)]
    fn require_plain(&self) -> Result<(), Error> {
        match self {
            LogFormat::Plain => Ok(()),
            _ => Err(Error::PlainFormatRequired { specified: *self }),
        }
    }

    fn to_subscriber_builder(&self) -> SubscriberBuilder {
        match self {
            LogFormat::Plain => tracing_subscriber::fmt(),
            LogFormat::Json => tracing_subscriber::fmt().json(),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    PlainFormatRequired {
        specified: LogFormat,
    },
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
            #[cfg(feature = "gelf")]
            Error::GelfConnectionFailed(err) => Some(err),
            Error::FileError(err) => Some(err),
        }
    }
}
