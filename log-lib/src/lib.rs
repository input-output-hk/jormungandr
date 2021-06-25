//! Opinionated log setup library for applications
//!
//! Example:
//!
//! ```
//! use log_lib::*;
//! use tracing::level_filters::LevelFilter;
//! use structopt::StructOpt;
//! use tracing::{ span, Level };
//!
//! // CliSettings::from_args in a real application
//! let cli = CliSettings::from_iter([
//!     "example",
//!     "--log-level",
//!     "trace",
//! ]);
//!
//! let file: FileSettings = serde_yaml::from_str(
//!     r#"
//!     level: info
//!     format: json
//!     "#,
//! )
//! .unwrap();
//!
//! let settings = LogSettings::new(&cli, Some(file));
//!
//! let (_guards, log_info) = settings.init_log().unwrap();
//!
//! let init_span = span!(Level::TRACE, "example", kind = "init");
//! let _enter = init_span.enter();
//!
//! if let Some(msgs) = log_info {
//!     // if log settings were overriden, we will have an info
//!     // message which we can unpack at this point.
//!     for msg in &msgs {
//!         tracing::info!("{}", msg);
//!     }
//! }
//! ```

use std::fmt::{self, Display};
use std::fs;
use std::io;
#[cfg(feature = "gelf")]
use std::net::SocketAddr;
use std::str::FromStr;
use tracing::level_filters::LevelFilter;
use tracing::subscriber::SetGlobalDefaultError;
use tracing_appender::non_blocking::WorkerGuard;
#[allow(unused_imports)]
use tracing_subscriber::layer::SubscriberExt;

use lazy_static::lazy_static;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use structopt::StructOpt;

const DEFAULT_FILTER_LEVEL: LevelFilter = LevelFilter::TRACE;
const DEFAULT_LOG_FORMAT: LogFormat = LogFormat::Default;
const DEFAULT_LOG_OUTPUT: LogOutput = LogOutput::Stderr;
const DEFAULT_LOG_SETTINGS_ENTRY: LogSettingsEntry = LogSettingsEntry {
    level: DEFAULT_FILTER_LEVEL,
    format: DEFAULT_LOG_FORMAT,
    output: DEFAULT_LOG_OUTPUT,
};

lazy_static! {
    static ref LOG_FILTER_LEVEL_POSSIBLE_VALUES: Vec<&'static str> = {
        [
            tracing::metadata::LevelFilter::OFF,
            tracing::metadata::LevelFilter::TRACE,
            tracing::metadata::LevelFilter::DEBUG,
            tracing::metadata::LevelFilter::INFO,
            tracing::metadata::LevelFilter::WARN,
            tracing::metadata::LevelFilter::ERROR,
        ]
        .iter()
        .map(|name| name.to_string().to_ascii_lowercase())
        .map(|name| &*Box::leak(name.into_boxed_str()))
        .collect()
    };
}

pub struct LogSettings {
    pub config: LogSettingsEntry,
    pub msgs: LogInfoMsg,
}

/// A wrapper to return an optional string message that we
/// have to manually log with `info!`, we need this because
/// some code executes before the logs are initialized.
pub type LogInfoMsg = Option<Vec<String>>;

#[derive(Clone, Debug, PartialEq)]
pub struct LogSettingsEntry {
    pub level: LevelFilter,
    pub format: LogFormat,
    pub output: LogOutput,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// Format of the logger.
pub enum LogFormat {
    Default,
    Plain,
    Json,
}

impl Default for LogFormat {
    fn default() -> Self {
        LogFormat::Default
    }
}

impl Display for LogFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            LogFormat::Default => "default",
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
    File(PathBuf),
    #[cfg(feature = "systemd")]
    Journald,
    #[cfg(feature = "gelf")]
    Gelf {
        backend: SocketAddr,
        log_id: String,
    },
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.trim().to_lowercase() {
            "plain" => Ok(LogFormat::Plain),
            "json" => Ok(LogFormat::Json),
            "default" => Ok(LogFormat::Default),
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
    pub fn new(command_line: &CliSettings, file: Option<FileSettings>) -> LogSettings {
        // Start with default config
        let mut log_config = DEFAULT_LOG_SETTINGS_ENTRY;
        let mut info_msgs: Vec<String> = Vec::new();

        //  Read log settings from the config file path.
        if let Some(cfg) = file.as_ref() {
            if let Some(level) = cfg.level {
                log_config.level = level;
            }
            if let Some(format) = cfg.format {
                log_config.format = format;
            }
            if let Some(output) = &cfg.output {
                log_config.output = output.clone();
            }
        }

        // If the command line specifies log arguments, they override everything
        // else.
        if let Some(output) = &command_line.log_output {
            if &log_config.output != output {
                info_msgs.push(format!(
                    "log output overriden from command line: {:?} replaced with {:?}",
                    log_config.output, output
                ));
            }
            log_config.output = output.clone();
        }
        if let Some(level) = command_line.log_level {
            if log_config.level != level {
                info_msgs.push(format!(
                    "log level overriden from command line: {:?} replaced with {:?}",
                    log_config.level, level
                ));
            }
            log_config.level = level;
        }
        if let Some(format) = command_line.log_format {
            if log_config.format != format {
                info_msgs.push(format!(
                    "log format overriden from command line: {:?} replaced with {:?}",
                    log_config.format, format
                ));
            }
            log_config.format = format;
        }

        let log_info_msg: LogInfoMsg = if info_msgs.is_empty() {
            None
        } else {
            Some(info_msgs)
        };

        LogSettings {
            config: log_config,
            msgs: log_info_msg,
        }
    }

    pub fn init_log(self) -> Result<(Vec<WorkerGuard>, LogInfoMsg), Error> {
        use tracing_subscriber::prelude::*;

        // Worker guards that need to be held on to.
        let mut guards = Vec::new();

        // configure the registry subscriber as the global default,
        // panics if something goes wrong.
        match self.config.output {
            LogOutput::Stdout => {
                let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
                guards.push(guard);
                match self.config.format {
                    LogFormat::Default | LogFormat::Plain => {
                        let layer = tracing_subscriber::fmt::Layer::new()
                            .with_level(true)
                            .with_writer(non_blocking);
                        tracing_subscriber::registry()
                            .with(self.config.level)
                            .with(layer)
                            .init();
                    }
                    LogFormat::Json => {
                        let layer = tracing_subscriber::fmt::Layer::new()
                            .json()
                            .with_level(true)
                            .with_writer(non_blocking);
                        tracing_subscriber::registry()
                            .with(self.config.level)
                            .with(layer)
                            .init();
                    }
                }
            }
            LogOutput::Stderr => {
                let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stderr());
                guards.push(guard);
                match self.config.format {
                    LogFormat::Default | LogFormat::Plain => {
                        let layer = tracing_subscriber::fmt::Layer::new()
                            .with_level(true)
                            .with_writer(non_blocking);
                        tracing_subscriber::registry()
                            .with(self.config.level)
                            .with(layer)
                            .init();
                    }
                    LogFormat::Json => {
                        let layer = tracing_subscriber::fmt::Layer::new()
                            .json()
                            .with_level(true)
                            .with_writer(non_blocking);
                        tracing_subscriber::registry()
                            .with(self.config.level)
                            .with(layer)
                            .init();
                    }
                }
            }
            LogOutput::File(path) => {
                let file = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(&path)
                    .map_err(|cause| Error::FileError {
                        path: path.clone(),
                        cause,
                    })?;
                let (non_blocking, guard) = tracing_appender::non_blocking(file);
                guards.push(guard);

                match self.config.format {
                    LogFormat::Default | LogFormat::Plain => {
                        let layer = tracing_subscriber::fmt::Layer::new()
                            .with_level(true)
                            .with_writer(non_blocking);
                        tracing_subscriber::registry()
                            .with(self.config.level)
                            .with(layer)
                            .init();
                    }
                    LogFormat::Json => {
                        let layer = tracing_subscriber::fmt::Layer::new()
                            .json()
                            .with_level(true)
                            .with_writer(non_blocking);
                        tracing_subscriber::registry()
                            .with(self.config.level)
                            .with(layer)
                            .init();
                    }
                }
            }
            #[cfg(feature = "systemd")]
            LogOutput::Journald => {
                self.config.format.require_default()?;
                let layer = tracing_journald::layer().map_err(Error::Journald)?;
                tracing_subscriber::registry()
                    .with(self.config.level)
                    .with(layer)
                    .init();
            }
            #[cfg(feature = "gelf")]
            LogOutput::Gelf { backend, .. } => {
                let (layer, task) = tracing_gelf::Logger::builder()
                    .connect_tcp(backend)
                    .map_err(Error::Gelf)?;
                tokio::spawn(task);
                tracing_subscriber::registry()
                    .with(self.config.level)
                    .with(layer)
                    .init();
            }
        }

        Ok((guards, self.msgs))
    }
}

impl LogFormat {
    #[allow(dead_code)]
    fn require_default(&self) -> Result<(), Error> {
        match self {
            LogFormat::Default => Ok(()),
            _ => Err(Error::FormatNotSupported { specified: *self }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("log format `{specified}` is not supported for this output")]
    FormatNotSupported { specified: LogFormat },
    #[error("failed to open the log file `{}`", .path.to_string_lossy())]
    FileError {
        path: PathBuf,
        #[source]
        cause: io::Error,
    },
    #[cfg(feature = "systemd")]
    #[error("cannot open journald socket")]
    Journald(#[source] io::Error),
    #[cfg(feature = "gelf")]
    #[error("GELF connection failed")]
    Gelf(tracing_gelf::BuilderError),
    #[error("failed to set global subscriber")]
    SetGlobalSubscriberError(#[source] SetGlobalDefaultError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileSettings {
    #[serde(with = "filter_level_opt_serde")]
    pub level: Option<LevelFilter>,
    pub format: Option<LogFormat>,
    pub output: Option<LogOutput>,
}

mod filter_level_opt_serde {
    use super::*;

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<LevelFilter>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(|variant| {
                variant.parse().map_err(|_| {
                    D::Error::unknown_variant(&variant, &**LOG_FILTER_LEVEL_POSSIBLE_VALUES)
                })
            })
            .transpose()
    }

    pub fn serialize<S: Serializer>(
        data: &Option<LevelFilter>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        data.map(|level| level.to_string()).serialize(serializer)
    }
}

fn log_level_parse(level: &str) -> Result<LevelFilter, String> {
    level
        .parse()
        .map_err(|_| format!("Unknown log level value: '{}'", level))
}

#[derive(Debug, StructOpt)]
#[structopt(name = "config")]
pub struct CliSettings {
    /// Set log messages minimum severity. If not configured anywhere, defaults to "info".
    #[structopt(
        long = "log-level",
        parse(try_from_str = log_level_parse),
        possible_values = &LOG_FILTER_LEVEL_POSSIBLE_VALUES
    )]
    pub log_level: Option<LevelFilter>,

    /// Set format of the log emitted. Can be "json" or "plain".
    /// If not configured anywhere, defaults to "plain".
    #[structopt(long = "log-format", parse(try_from_str))]
    pub log_format: Option<LogFormat>,

    /// Set format of the log emitted. Can be "stdout", "stderr",
    /// "syslog" (Unix only) or "journald"
    /// (linux with systemd only, must be enabled during compilation).
    /// If not configured anywhere, defaults to "stderr".
    #[structopt(long = "log-output", parse(try_from_str))]
    pub log_output: Option<LogOutput>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn cli_has_priority() {
        let cli = CliSettings::from_iter(vec![
            "example",
            "--log-level",
            &LevelFilter::TRACE.to_string(),
        ]);

        let file: FileSettings = serde_yaml::from_str(
            r#"
            level: info
            output:
                file:
                    output.log
            "#,
        )
        .unwrap();

        let settings = LogSettings::new(&cli, Some(file));

        assert_eq!(settings.config.level, LevelFilter::TRACE);
        assert_eq!(settings.config.output, LogOutput::File("output.log".into()));
        assert_eq!(settings.config.format, DEFAULT_LOG_FORMAT);

        let (_guards, _log_info) = settings.init_log().unwrap();
    }
}
