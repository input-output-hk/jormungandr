use std::error;
use std::fmt::{self, Display};
use std::fs;
use std::io;
use std::ops::Deref;
use std::str::FromStr;

use tracing::{level_filters::LevelFilter, Event, Id, Metadata, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;
#[cfg(feature = "gelf")]
use tracing_gelf::Gelf;

use tracing::span::{Attributes, Record};
use tracing_subscriber::fmt::format::Format;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::layer::{Layer, Layered, SubscriberExt};
use tracing_subscriber::Registry;

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
    pub fn init_log(mut self) -> Result<Vec<WorkerGuard>, Error> {
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
            tracing::subscriber::set_global_default(init_layer);
        }

        Ok(guards)
    }
}

impl LogSettingsEntry {
    fn to_subscriber(
        &self,
    ) -> Result<
        (
            Box<dyn Subscriber + Send + Sync>,
            Option<tracing_appender::non_blocking::WorkerGuard>,
        ),
        Error,
    > {
        let Self {
            output,
            level,
            format,
        } = &self;

        let builder = tracing_subscriber::fmt::SubscriberBuilder::default();

        match output {
            LogOutput::Stdout => {
                let (subscriber, guard) = tracing_appender::non_blocking(std::io::stdout());
                let builder = builder.with_writer(subscriber).with_max_level(*level);
                match format {
                    LogFormat::Plain => Ok((Box::new(builder.finish()), Some(guard))),
                    LogFormat::Json => Ok((Box::new(builder.json().finish()), Some(guard))),
                }
            }
            LogOutput::Stderr => {
                let (subscriber, guard) = tracing_appender::non_blocking(std::io::stderr());
                let builder = builder.with_writer(subscriber).with_max_level(*level);
                match format {
                    LogFormat::Plain => Ok((Box::new(builder.finish()), Some(guard))),
                    LogFormat::Json => Ok((Box::new(builder.json().finish()), Some(guard))),
                }
            }
            #[cfg(feature = "systemd")]
            LogOutput::Journald => {
                let layer = tracing_journald::layer()?;
                format.require_plain()?;
                Ok((
                    Box::new(builder.with_max_level(level).with_subscriber(layer)),
                    None,
                ))
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
                Ok((Box::new(subscriber), None))
            }
            LogOutput::File(path) => {
                let file = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(path)
                    .map_err(Error::FileError)?;
                let (subscriber, guard) = tracing_appender::non_blocking(file);
                let builder = builder.with_writer(subscriber).with_max_level(*level);
                match format {
                    LogFormat::Plain => Ok((Box::new(builder.finish()), Some(guard))),
                    LogFormat::Json => Ok((Box::new(builder.json().finish()), Some(guard))),
                }
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
