#[cfg(feature = "gelf")]
use std::net::SocketAddr;
use std::{
    fmt::{self, Display},
    fs, io,
    path::PathBuf,
    str::FromStr,
};
use tracing::{level_filters::LevelFilter, subscriber::SetGlobalDefaultError};
use tracing_appender::non_blocking::WorkerGuard;
#[allow(unused_imports)]
use tracing_subscriber::layer::SubscriberExt;

pub struct LogSettings {
    pub config: LogSettingsEntry,
    pub msgs: LogInfoMsg,
}

/// A wrapper to return an optional string message that we
/// have to manually log with `info!`, we need this because
/// some code executes before the logs are initialized.
pub type LogInfoMsg = Option<Vec<String>>;

#[derive(Clone, Debug, PartialEq, Eq)]
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
                            .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
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
                            .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
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
                            .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
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
