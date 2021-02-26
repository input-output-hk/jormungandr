use std::fmt::{self, Display};
use std::fs;
use std::io::{self};
#[cfg(feature = "gelf")]
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;

use tracing::subscriber::SetGlobalDefaultError;
#[allow(unused_imports)]
use tracing_subscriber::layer::SubscriberExt;

pub struct LogSettings(pub Vec<LogSettingsEntry>, pub LogInfoMsg);

/// A wrapper to return an optional string message that we
/// have to manually log with `info!`, we need this because
/// some code executes before the logs are initialized.
pub type LogInfoMsg = Option<String>;

#[derive(Clone, Debug)]
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

// Settings for output layers.
#[derive(Default)]
struct LogOutputLayerSettings {
    stdout: Option<LogSettingsEntry>,
    stderr: Option<LogSettingsEntry>,
    file: Option<LogSettingsEntry>,
    #[cfg(feature = "systemd")]
    journald: Option<LogSettingsEntry>,
    #[cfg(feature = "gelf")]
    gelf: Option<LogSettingsEntry>,
}

impl LogOutputLayerSettings {
    // Overwrites settings by LogOutput variant, wrapping
    // log settings entry into and Option
    fn read_setting(&mut self, setting: LogSettingsEntry) {
        match setting.output {
            LogOutput::Stdout => {
                self.stdout = Some(setting);
            }
            LogOutput::Stderr => {
                self.stderr = Some(setting);
            }
            LogOutput::File(_) => {
                self.file = Some(setting);
            }
            #[cfg(feature = "systemd")]
            LogOutput::Journald => {
                self.journald = Some(setting);
            }
            #[cfg(feature = "gelf")]
            LogOutput::Gelf { .. } => {
                self.gelf = Some(setting);
            }
        }
    }
}

impl LogSettings {
    pub fn init_log(self) -> Result<(Vec<WorkerGuard>, LogInfoMsg), Error> {
        use tracing_subscriber::prelude::*;
        let mut guards = Vec::new();

        let registry = tracing_subscriber::registry();

        // Parse which settings are present for possible outputs
        let mut layer_settings = LogOutputLayerSettings::default();
        for config in self.0.into_iter() {
            layer_settings.read_setting(config);
        }
        let (std_out_layer, std_out_layer_json) = if let Some(settings) = layer_settings.stdout {
            let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
            guards.push(guard);
            match settings.format {
                LogFormat::Default | LogFormat::Plain => {
                    let layer = tracing_subscriber::fmt::Layer::new().with_writer(non_blocking);
                    (Some(layer), None)
                }
                LogFormat::Json => {
                    let layer = tracing_subscriber::fmt::Layer::new()
                        .json()
                        .with_writer(non_blocking);
                    (None, Some(layer))
                }
            }
        } else {
            (None, None)
        };
        let (std_err_layer, std_err_layer_json) = if let Some(settings) = layer_settings.stderr {
            let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stderr());
            guards.push(guard);
            match settings.format {
                LogFormat::Default | LogFormat::Plain => {
                    let layer = tracing_subscriber::fmt::Layer::new().with_writer(non_blocking);
                    (Some(layer), None)
                }
                LogFormat::Json => {
                    let layer = tracing_subscriber::fmt::Layer::new()
                        .json()
                        .with_writer(non_blocking);
                    (None, Some(layer))
                }
            }
        } else {
            (None, None)
        };
        let (file_layer, file_layer_json) = if let Some(settings) = layer_settings.file {
            // have to use if let because it's an enum
            if let LogOutput::File(path) = settings.output {
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
                match settings.format {
                    LogFormat::Default | LogFormat::Plain => {
                        let layer = tracing_subscriber::fmt::Layer::new().with_writer(non_blocking);
                        (Some(layer), None)
                    }
                    LogFormat::Json => {
                        let layer = tracing_subscriber::fmt::Layer::new()
                            .json()
                            .with_writer(non_blocking);
                        (None, Some(layer))
                    }
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        #[cfg(feature = "systemd")]
        let journald_layer = if let Some(settings) = layer_settings.journald {
            settings.format.require_default()?;
            let layer = tracing_journald::layer().map_err(Error::Journald)?;
            Some(layer)
        } else {
            None
        };
        #[cfg(feature = "gelf")]
        let gelf_layer = if let Some(settings) = layer_settings.gelf {
            // have to use if let because it's an enum
            if let LogOutput::Gelf { backend, .. } = settings.output {
                let (layer, task) = tracing_gelf::Logger::builder()
                    .connect_tcp(backend)
                    .map_err(Error::Gelf)?;
                tokio::spawn(task);
                Some(layer)
            } else {
                None
            }
        } else {
            None
        };

        // configure the registry with optional outputs configured above
        let registry = registry.with(std_out_layer_json).with(std_out_layer);
        let registry = registry.with(std_err_layer_json).with(std_err_layer);
        let registry = registry.with(file_layer_json).with(file_layer);
        #[cfg(feature = "systemd")]
        let registry = registry.with(journald_layer);
        #[cfg(feature = "gelf")]
        let registry = registry.with(gelf_layer);

        // configure the registry subscriber as the global default,
        // panics if something goes wrong.
        registry.init();

        Ok((guards, self.1))
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
