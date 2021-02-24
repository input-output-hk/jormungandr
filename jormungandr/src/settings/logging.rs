use std::fmt::{self, Display};
use std::fs;
use std::io::{self, Write};
#[cfg(feature = "gelf")]
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use tracing::{level_filters::LevelFilter, Event, Id, Metadata, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;

use tracing::span::{Attributes, Record};
use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::fmt::SubscriberBuilder;
#[allow(unused_imports)]
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::layer::{Layer, Layered};

pub struct LogSettings(pub Vec<LogSettingsEntry>, pub LogInfoMsg);

/// A wrapper to return an optional string message that we
/// have to manually log with `info!`, we need this because
/// some code executes before the logs are initialized.
pub type LogInfoMsg = Option<String>;

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

struct BoxedSubscriber(Box<dyn Subscriber + Send + Sync>);

impl Subscriber for BoxedSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.0.enabled(metadata)
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        self.0.new_span(span)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        self.0.record(span, values)
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.0.record_follows_from(span, follows)
    }

    fn event(&self, event: &Event<'_>) {
        self.0.event(event)
    }

    fn enter(&self, span: &Id) {
        self.0.enter(span)
    }

    fn exit(&self, span: &Id) {
        self.0.exit(span)
    }
}

impl Layer<BoxedSubscriber> for BoxedSubscriber {}

impl LogSettings {
    pub fn init_log(self) -> Result<(Vec<WorkerGuard>, LogInfoMsg), Error> {
        // WIP: Replacing this code
        //
        // * Use tracing_subscriber::registry::Registry instead of
        //   boxing subscribers
        // * Create specific layer types for each output format, implement
        //   ````
        //   impl<S> tracing_subscriber::Layer<S> for OutputLayer
        //   where
        //       S: Subscriber + for<'span> LookupSpan<'span>
        //   ```
        //   * implement uniform process of composing Layer and Subscriber types
        //     for each output format
        use tracing_subscriber::prelude::*;
        let mut guards = Vec::new();
        let mut layers: Vec<Layered<_, BoxedSubscriber>> = Vec::new();
        for config in self.0.into_iter() {
            let (subscriber, guard) = config.to_subscriber()?;
            let subscriber = BoxedSubscriber(subscriber);

            let layer: Layered<_, _, BoxedSubscriber> =
                tracing_subscriber::layer::Identity::new().with_subscriber(subscriber);

            layers.push(layer);
            if let Some(guard) = guard {
                guards.push(guard);
            }
        }

        let mut layer_iter = layers.into_iter();
        if let Some(layer) = layer_iter.next() {
            let mut init_layer: BoxedSubscriber = BoxedSubscriber(Box::new(layer));
            for layer in layer_iter {
                init_layer = BoxedSubscriber(Box::new(init_layer.with(layer)));
            }
            tracing::subscriber::set_global_default(init_layer)
                .map_err(Error::SetGlobalSubscriberError)?;
        }

        Ok((guards, self.1))
    }
}

impl LogSettingsEntry {
    fn to_subscriber(
        &self,
    ) -> Result<(Box<dyn Subscriber + Send + Sync>, Option<WorkerGuard>), Error> {
        let Self {
            output,
            level,
            format,
        } = self;

        let builder = SubscriberBuilder::default();

        match output {
            LogOutput::Stdout => {
                let (subscriber, guard) = tracing_appender::non_blocking(std::io::stdout());
                let builder = builder.with_writer(subscriber).with_max_level(*level);
                let subscriber: Box<dyn Subscriber + Send + Sync> = match format {
                    LogFormat::Default | LogFormat::Plain => Box::new(builder.finish()),
                    LogFormat::Json => Box::new(builder.json().finish()),
                };
                Ok((subscriber, Some(guard)))
            }
            LogOutput::Stderr => {
                let (subscriber, guard) = tracing_appender::non_blocking(std::io::stderr());
                let builder = builder.with_writer(subscriber).with_max_level(*level);
                let subscriber: Box<dyn Subscriber + Send + Sync> = match format {
                    LogFormat::Default | LogFormat::Plain => Box::new(builder.finish()),
                    LogFormat::Json => Box::new(builder.json().finish()),
                };
                Ok((subscriber, Some(guard)))
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
                let (subscriber, guard) = tracing_appender::non_blocking(file);
                let builder = builder.with_writer(subscriber).with_max_level(*level);
                let subscriber: Box<dyn Subscriber + Send + Sync> = match format {
                    LogFormat::Default | LogFormat::Plain => Box::new(builder.finish()),
                    LogFormat::Json => Box::new(builder.json().finish()),
                };
                Ok((subscriber, Some(guard)))
            }
            #[cfg(feature = "systemd")]
            LogOutput::Journald => {
                format.require_default()?;
                let layer = tracing_journald::layer().map_err(Error::Journald)?;
                let subscriber = builder.with_max_level(*level).finish().with(layer);
                Ok((Box::new(subscriber), None))
            }
            #[cfg(feature = "gelf")]
            LogOutput::Gelf {
                backend: address,
                log_id: _graylog_source,
            } => {
                format.require_default()?;
                // TODO: maybe handle this tasks outside somehow.
                let (layer, task) = tracing_gelf::Logger::builder()
                    .connect_tcp(address.clone())
                    .map_err(Error::Gelf)?;
                tokio::spawn(task);
                let subscriber = builder.with_max_level(*level).finish().with(layer);
                Ok((Box::new(subscriber), None))
            }
        }
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
