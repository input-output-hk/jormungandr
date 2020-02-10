mod control_command;
mod monitor;

pub(crate) use self::control_command::{ControlCommand, Reply};
pub use self::{
    control_command::{ControlHandler, WatchdogQuery},
    monitor::WatchdogMonitor,
};
use crate::service::{ServiceError, ServiceIdentifier, StatusReport};
use async_trait::async_trait;
use std::{any::Any, fmt};
use thiserror::Error;
use tokio::{
    runtime::Runtime,
    sync::{mpsc, oneshot},
};

/// trait to define the different core services and their
/// associated metadata
#[async_trait]
pub trait CoreServices: Send + Sync {
    type Settings: Default + Send + serde::ser::Serialize + serde::de::DeserializeOwned;

    fn add_cli_args<'a, 'b>(app: clap::App<'a, 'b>) -> clap::App<'a, 'b>
    where
        'a: 'b;

    fn new<'a>(settings: &mut Self::Settings, args: &clap::ArgMatches<'a>) -> (Vec<Runtime>, Self);

    fn stop(&mut self, service_identifier: ServiceIdentifier) -> Result<(), WatchdogError>;
    async fn status(
        &mut self,
        service_identifier: ServiceIdentifier,
    ) -> Result<StatusReport, WatchdogError>;
    fn start(
        &mut self,
        service_identifier: ServiceIdentifier,
        watchdog_query: WatchdogQuery,
    ) -> Result<(), WatchdogError>;
    fn intercoms(
        &mut self,
        service_identifier: ServiceIdentifier,
    ) -> Result<Box<dyn Any + Send + 'static>, WatchdogError>;
}

pub struct Watchdog<T: CoreServices> {
    services: T,
    settings: T::Settings,
    on_drop_send: oneshot::Sender<()>,
}

pub struct WatchdogBuilder<'a, 'b, T>
where
    T: CoreServices,
    'a: 'b,
{
    app: clap::App<'a, 'b>,
    _marker: std::marker::PhantomData<T>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WatchdogError {
    #[error("Unknown service {service_identifier}, available services are {possible_values:?}")]
    UnknownService {
        service_identifier: ServiceIdentifier,
        possible_values: &'static [ServiceIdentifier],
    },

    #[error("Cannot start service {service_identifier}: {source}")]
    CannotStartService {
        service_identifier: ServiceIdentifier,
        source: ServiceError,
    },

    #[error("Cannot connect to service {service_identifier}, service might be shutdown")]
    CannotConnectToService {
        service_identifier: ServiceIdentifier,
        retry_attempted: bool,
    },
}

const APP_ARG_CONFIG_FILE: &str = "WATCHDOG_SERVICES_CONFIG_FILE";

impl<'a, 'b, T> WatchdogBuilder<'a, 'b, T>
where
    'a: 'b,
    T: CoreServices,
{
    pub fn new(app: clap::App<'a, 'b>) -> Self {
        let app = app.arg(
            clap::Arg::with_name(APP_ARG_CONFIG_FILE)
                .short("c")
                .long("config")
                .takes_value(true)
                .value_name("FILE")
                .help("Path to the application's configuration file")
                .long_help(
                    "Path to the application's configuration file. The default is to search for
a configuration file in the current directory. However it is preferable to
give the absolute path to the file.",
                )
                .global(true)
                .env(crate_name!())
                .default_value("config.yaml"),
        );

        Self {
            app,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn build(self) -> WatchdogMonitor
    where
        T: CoreServices + 'static,
    {
        let app = T::add_cli_args(self.app);

        let args = app.get_matches();

        Self::build_(args)
    }

    pub fn build_from_safe<I, V>(self, itr: I) -> WatchdogMonitor
    where
        T: CoreServices + 'static,
        I: IntoIterator<Item = V>,
        V: Into<::std::ffi::OsString> + Clone,
    {
        let app = T::add_cli_args(self.app);

        let args = app.get_matches_from_safe(itr).unwrap();

        Self::build_(args)
    }

    fn build_(args: clap::ArgMatches<'a>) -> WatchdogMonitor
    where
        T: CoreServices + 'static,
    {
        let config_path = value_t!(args.value_of(APP_ARG_CONFIG_FILE), std::path::PathBuf)
            .unwrap_or_else(|e| e.exit());

        // TODO: handle the case where there is no config file to read?
        let mut settings = if let Ok(file) = std::fs::File::open(&config_path) {
            serde_yaml::from_reader(file).unwrap()
        } else {
            T::Settings::default()
        };

        let (runtimes, services) = T::new(&mut settings, &args);

        let (sender, receiver) = mpsc::channel(10);
        let (on_drop_send, on_drop_receive) = oneshot::channel();

        let watchdog = Watchdog {
            on_drop_send,
            services,
            settings,
        };

        let rt = tokio::runtime::Builder::new()
            .enable_all()
            .thread_name("watchdog")
            .threaded_scheduler()
            .build()
            .unwrap();

        let query = WatchdogQuery::new(rt.handle().clone(), sender.clone());

        rt.spawn(async move { watchdog.watchdog(receiver, query).await });

        WatchdogMonitor::new(rt, runtimes, sender, on_drop_receive)
    }
}

impl<T> Watchdog<T>
where
    T: CoreServices,
{
    #[tracing::instrument(skip(self, cc, watchdog_query), target = "watchdog", level = "info")]
    async fn watchdog(
        mut self,
        mut cc: mpsc::Receiver<ControlCommand>,
        watchdog_query: WatchdogQuery,
    ) {
        while let Some(command) = cc.recv().await {
            match command {
                ControlCommand::Shutdown | ControlCommand::Kill => {
                    // TODO: for now we assume shutdown and kill are the same
                    //       but on the long run it will need to send a Shutdown
                    //       signal to every services so they can save state and
                    //       release resources properly

                    tracing::warn!("stopping watchdog");
                    break;
                }
                ControlCommand::Status {
                    service_identifier,
                    reply,
                } => {
                    let status_report = self.services.status(service_identifier).await;
                    if let Ok(status_report) = &status_report {
                        tracing::info!(
                            %status_report.identifier,
                            status_report.number_restart = status_report.started,
                            %status_report.status,
                            %status_report.intercom.number_sent,
                            %status_report.intercom.number_received,
                            %status_report.intercom.number_connections,
                            %status_report.intercom.processing_speed_mean,
                            %status_report.intercom.processing_speed_variance,
                            %status_report.intercom.processing_speed_standard_derivation,
                        );
                    }
                    reply.reply(status_report);
                }
                ControlCommand::Start {
                    service_identifier,
                    reply,
                } => {
                    tracing::info!(%service_identifier, "start");
                    reply.reply(
                        self.services
                            .start(service_identifier, watchdog_query.clone()),
                    );
                }
                ControlCommand::Stop {
                    service_identifier,
                    reply,
                } => {
                    tracing::info!(%service_identifier, "stop");
                    reply.reply(self.services.stop(service_identifier));
                }
                ControlCommand::Intercom {
                    service_identifier,
                    reply,
                } => {
                    tracing::trace!(%service_identifier, "query intercom");
                    // TODO: surround the operation with a timeout and
                    //       result to success
                    reply.reply(self.services.intercoms(service_identifier));
                }
            }
        }

        if self.on_drop_send.send(()).is_err() {
            // ignore error for now
        }
    }
}

impl<T: CoreServices> fmt::Debug for Watchdog<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Watchdog").finish()
    }
}
