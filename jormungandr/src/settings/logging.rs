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

pub struct LogSettings(pub Vec<LogSettingsEntry>);

#[derive(Debug)]
pub struct LogSettingsEntry {
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
    #[cfg(unix)]
    SyslogUdp {
        host: String,
        hostname: String,
    },
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

#[derive(Debug)]
struct DrainMux<D>(Vec<D>);

impl<D> DrainMux<D> {
    pub fn new(d: Vec<D>) -> Self {
        Self(d)
    }
}

impl<D: Drain> Drain for DrainMux<D> {
    type Ok = ();
    type Err = D::Err;

    fn log(
        &self,
        record: &slog::Record,
        values: &slog::OwnedKVList,
    ) -> Result<Self::Ok, Self::Err> {
        self.0
            .iter()
            .try_for_each(|drain| drain.log(record, values).map(|_| ()))
    }
}

/// slog serializers do not care about duplicates in KV fields that can occur
/// under certain circumstances. This serializer serves as a wrapper on top of
/// the actual serializer that takes care of duplicates. First it checks if the
/// same key was already. If so, this key is skipped during the serialization.
/// Otherwise the KV pair is passed to the inner serializer.
struct DedupSerializer<'a> {
    inner: &'a mut dyn slog::Serializer,
    seen_keys: std::collections::HashSet<slog::Key>,
}

impl<'a> DedupSerializer<'a> {
    fn new(inner: &'a mut dyn slog::Serializer) -> Self {
        Self {
            inner,
            seen_keys: Default::default(),
        }
    }
}

macro_rules! dedup_serializer_method_impl {
    ($(#[$m:meta])* $t:ty => $f:ident) => {
        $(#[$m])*
        fn $f(&mut self, key : slog::Key, val : $t)
            -> slog::Result {
                if self.seen_keys.contains(&key) {
                    return Ok(())
                }
                self.seen_keys.insert(key.clone());
                self.inner.$f(key, val)
            }
    };
}

impl<'a> slog::Serializer for DedupSerializer<'a> {
    dedup_serializer_method_impl! {
        &fmt::Arguments => emit_arguments
    }
    dedup_serializer_method_impl! {
        usize => emit_usize
    }
    dedup_serializer_method_impl! {
        isize => emit_isize
    }
    dedup_serializer_method_impl! {
        bool => emit_bool
    }
    dedup_serializer_method_impl! {
        char => emit_char
    }
    dedup_serializer_method_impl! {
        u8 => emit_u8
    }
    dedup_serializer_method_impl! {
        i8 => emit_i8
    }
    dedup_serializer_method_impl! {
        u16 => emit_u16
    }
    dedup_serializer_method_impl! {
        i16 => emit_i16
    }
    dedup_serializer_method_impl! {
        u32 => emit_u32
    }
    dedup_serializer_method_impl! {
        i32 => emit_i32
    }
    dedup_serializer_method_impl! {
        u64 => emit_u64
    }
    dedup_serializer_method_impl! {
        i64 => emit_i64
    }
    dedup_serializer_method_impl! {
        #[cfg(integer128)]
        u128 => emit_u128
    }
    dedup_serializer_method_impl! {
        #[cfg(integer128)]
        i128 => emit_i128
    }
    dedup_serializer_method_impl! {
        &str => emit_str
    }

    fn emit_unit(&mut self, key: slog::Key) -> slog::Result {
        if self.seen_keys.contains(&key) {
            return Ok(());
        }
        self.seen_keys.insert(key.clone());
        self.inner.emit_unit(key)
    }

    fn emit_none(&mut self, key: slog::Key) -> slog::Result {
        if self.seen_keys.contains(&key) {
            return Ok(());
        }
        self.seen_keys.insert(key.clone());
        self.inner.emit_none(key)
    }
}

/// The wrapper on top of an arbitrary KV object that utilize DedupSerializer.
struct DedupKV<T>(T);

impl<T: slog::KV> slog::KV for DedupKV<T> {
    fn serialize(
        &self,
        record: &slog::Record,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        let mut serializer = DedupSerializer::new(serializer);
        self.0.serialize(&record, &mut serializer)
    }
}

/// slog drain that uses DedupKV to remove duplicate keys from KV lists
struct DedupDrain<D>(D);

impl<D: Drain> Drain for DedupDrain<D> {
    type Ok = D::Ok;
    type Err = D::Err;

    fn log(
        &self,
        record: &slog::Record,
        values: &slog::OwnedKVList,
    ) -> Result<Self::Ok, Self::Err> {
        // clone is ok here because the underlying data is Arc
        let values = slog::OwnedKV(DedupKV(values.clone()));
        self.0.log(record, &values.into())
    }
}

impl LogSettings {
    pub fn to_logger(&self) -> Result<Logger, Error> {
        let mut drains = Vec::new();
        for config in self.0.iter() {
            drains.push(config.to_logger()?);
        }
        let common_drain = DedupDrain(DrainMux::new(drains)).fuse();
        Ok(slog::Logger::root(common_drain, o!()))
    }
}

impl LogSettingsEntry {
    pub fn to_logger(&self) -> Result<slog::Filter<Async, impl slog::FilterFn>, Error> {
        let filter_level = self.level;
        let drain = self
            .output
            .to_logger(&self.format)?
            .filter(move |record| filter_level.accepts(record.level()));
        Ok(drain)
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
            #[cfg(unix)]
            LogOutput::SyslogUdp { host, hostname } => {
                use std::net::{IpAddr, Ipv4Addr, SocketAddr};

                format.require_plain()?;

                let mut local_port = 30_000;
                let host = host.parse().map_err(Error::SyslogInvalidHost)?;

                // automatically select local port
                loop {
                    let local =
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), local_port);
                    let res = slog_syslog::SyslogBuilder::new()
                        .facility(Facility::LOG_USER)
                        .udp(local, host, hostname)
                        .start();
                    match res {
                        Ok(drain) => break Ok(drain.into_async()),
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::AddrInUse && local_port < 65_535 {
                                local_port += 1;
                                continue;
                            }

                            break Err(Error::SyslogAccessFailed(e));
                        }
                    }
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
                Ok(gelf_drain.into_async())
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
    #[cfg(unix)]
    SyslogInvalidHost(std::net::AddrParseError),
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
            #[cfg(unix)]
            Error::SyslogInvalidHost(_) => write!(f, "invalid syslog host address"),
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
            #[cfg(unix)]
            Error::SyslogInvalidHost(err) => Some(err),
            #[cfg(feature = "gelf")]
            Error::GelfConnectionFailed(err) => Some(err),
            Error::FileError(err) => Some(err),
        }
    }
}
