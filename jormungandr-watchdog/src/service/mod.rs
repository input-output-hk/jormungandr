mod control;
mod intercom;
mod settings;
mod state;
mod stats;
mod status;

pub use self::{
    control::{Control, ControlReader, Controller},
    intercom::{
        Intercom, IntercomMsg, IntercomReceiver, IntercomSender, IntercomStats, IntercomStatus,
        NoIntercom,
    },
    settings::{NoSettings, Settings, SettingsReader, SettingsUpdater},
    state::{NoState, State, StateHandler, StateSaver},
    stats::Stats,
    status::{Status, StatusReader, StatusUpdater},
};
use crate::watchdog::WatchdogQuery;
use async_trait::async_trait;
use futures_util::future::abortable;
use std::future::Future;
use thiserror::Error;
use tokio::{
    runtime::{Builder, Handle, Runtime},
    task::JoinHandle,
};

pub type ServiceIdentifier = &'static str;

#[async_trait]
pub trait Service: Send + Sized + 'static {
    const SERVICE_IDENTIFIER: ServiceIdentifier;

    type State: State;
    type Settings: Settings;
    type Intercom: IntercomMsg;

    fn prepare(service_state: ServiceState<Self>) -> Self;

    async fn start(self);
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ServiceError {
    #[error("Service cannot be started because status is: {status}")]
    CannotStart { status: Status },
}

#[derive(Debug, Clone)]
pub struct StatusReport {
    pub identifier: ServiceIdentifier,
    pub status: Status,
    pub intercom: IntercomStatus,
    // add uptime
}

pub struct ServiceManager<T: Service> {
    identifier: ServiceIdentifier,

    settings: SettingsUpdater<T::Settings>,
    state: StateSaver<T::State>,
    intercom_sender: IntercomSender<T::Intercom>,
    intercom_stats: IntercomStats,

    status: StatusReader,
    controller: Controller,
    runtime: Runtime,
}

/// not to mistake for `tokio`'s runtime. This is the object that
/// will hold the service process and all the other associated data.
/// to allow for a good running activity of the service.
///
pub struct ServiceRuntime<T: Service> {
    service_state: ServiceState<T>,

    status: StatusUpdater,
    control: ControlReader,
}

/// this is the object that every services has access to
///
/// each service has its own ServiceState. It allows to connect to
/// other services [`intercom_with`] as well as getting access to the
/// service's settings or state
pub struct ServiceState<T: Service> {
    identifier: ServiceIdentifier,
    handle: Handle,
    settings: SettingsReader<T::Settings>,
    state: StateHandler<T::State>,
    intercom_receiver: IntercomReceiver<T::Intercom>,
    watchdog_query: WatchdogQuery,
}

impl<T: Service> ServiceState<T> {
    /// access the service's Identifier
    ///
    /// this is just similar to calling `<T as Service>::SERVICE_IDENTIFIER`
    pub fn identifier(&self) -> ServiceIdentifier {
        self.identifier
    }

    /// open an [`Intercom`] handle with the given service `O`
    ///
    /// [`Intercom`]: ./struct.Intercom.html
    pub fn intercom_with<O: Service>(&self) -> Intercom<O> {
        self.watchdog_query.intercom::<O>()
    }

    /// access the service's IntercomReceiver end
    ///
    /// this is the end that will receive intercom messages from other services
    pub fn intercom_mut(&mut self) -> &mut IntercomReceiver<T::Intercom> {
        &mut self.intercom_receiver
    }

    /// get the [`SettingsReader`] for the given running Service
    ///
    /// this from there one can "borrow" the settings or clone the reader
    ///
    /// [`SettingsReader`]: ./struct.SettingsReader.html
    pub fn settings(&self) -> &SettingsReader<T::Settings> {
        &self.settings
    }

    /// access the [`StateHandler`] of the running service
    ///
    /// this will allow to access or update the state of the service.
    /// Every time the state is updated, the watchdog is notified and can
    /// save a copy of the state for future uses.
    ///
    /// [`StateHandler`]: ./struct.StateHandler.html
    pub fn state(&self) -> &StateHandler<T::State> {
        &self.state
    }

    /// access the service's Runtime's handle
    ///
    /// This object can be cloned and send between tasks allowing for
    /// other tasks to create their own subtasks and so on
    pub fn runtime_handle(&self) -> &Handle {
        &self.handle
    }

    /// spawn the given future in the context of the Service's Runtime.
    ///
    /// While there is no way to enforce the users to actually spawn tasks
    /// within the Runtime we can at least urge the users to do so and avoid
    /// using the global runtime context as it may be used for other purposes.
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime_handle().spawn(future)
    }
}

impl<T: Service> ServiceManager<T> {
    pub async fn new() -> Self {
        let identifier = T::SERVICE_IDENTIFIER;

        let settings = SettingsUpdater::new(T::Settings::default()).await;
        let state = StateSaver::new(T::State::default()).await;
        let status = StatusReader::new(Status::shutdown());
        let controller = Controller::new().await;
        let (intercom_sender, _, intercom_stats) = intercom::channel();

        let runtime = Builder::new()
            .enable_io()
            .enable_time()
            .thread_name(identifier)
            .threaded_scheduler()
            .build()
            .unwrap();

        Self {
            identifier,
            settings,
            state,
            intercom_sender,
            intercom_stats,
            status,
            controller,
            runtime,
        }
    }

    pub fn intercom(&self) -> IntercomSender<T::Intercom> {
        self.intercom_sender.clone()
    }

    pub async fn status(&self) -> StatusReport {
        StatusReport {
            identifier: self.identifier,
            status: self.status.status(),
            intercom: self.intercom_stats.status().await,
        }
    }

    pub fn shutdown(&mut self) {
        match self.status.status() {
            Status::Shutdown { .. } | Status::ShuttingDown { .. } => {
                // Ignore as the node is either shutdown or already shutting
                // down
            }
            Status::Starting { .. } | Status::Started { .. } => {
                // send only if the node will have a chance to actually read
                // the command
                self.controller.send(Control::Shutdown)
            }
        }
    }

    pub fn runtime(
        &mut self,
        watchdog_query: WatchdogQuery,
    ) -> Result<ServiceRuntime<T>, ServiceError> {
        let status = self.status.status();
        if !status.is_shutdown() {
            Err(ServiceError::CannotStart { status })
        } else {
            let (intercom_sender, intercom_receiver, intercom_stats) =
                intercom::channel::<T::Intercom>();

            std::mem::replace(&mut self.intercom_sender, intercom_sender);
            std::mem::replace(&mut self.intercom_stats, intercom_stats);

            Ok(ServiceRuntime {
                service_state: ServiceState {
                    identifier: self.identifier,
                    handle: self.runtime.handle().clone(),
                    settings: self.settings.reader(),
                    state: self.state.handler(),
                    intercom_receiver,
                    watchdog_query,
                },
                status: self.status.updater(),
                control: self.controller.reader(),
            })
        }
    }
}

impl<T: Service> ServiceRuntime<T> {
    pub fn start(self) {
        let ServiceRuntime {
            service_state,
            status,
            mut control,
        } = self;

        status.update(Status::starting());

        let handle = service_state.handle.clone();
        let runner = T::prepare(service_state);

        let (runner, abort_handle) = abortable(async move { runner.start().await });

        let mut service_join_handle = handle.spawn(runner);

        // the runner (the service) has been started into its current runtime. They must use
        // the `handle` to spawn new tasks.
        //
        // however the control of the service is still spawned in the watchdog current context
        // so we can perform the management tasks without disrupting the service's runtime
        tokio::spawn(async move {
            status.update(Status::started());

            loop {
                tokio::select! {
                    join_result = &mut service_join_handle => {
                        if let Err(join_error) = join_result {
                            // TODO: the task could not join, either cancelled
                            //       or panicked. Ideally we need to document
                            //       this panic and see what kind of strategy
                            //       can be applied (can we restart the service?)
                            //       or is it a fatal panic and we cannot recover?

                            eprintln!(
                                "{}'s main process failed with following error {:#?}",
                                T::SERVICE_IDENTIFIER,
                                join_error
                            );

                        } else {
                            abort_handle.abort();
                        }
                        status.update(Status::shutdown());
                        break;
                    }
                    control = &mut control => {
                        match control {
                            Some(Control::Shutdown) => {
                                status.update(Status::shutting_down());
                                // TODO: send the shutdown signal to the task
                            }
                            None | Some(Control::Kill) => {
                                status.update(Status::shutdown());
                                abort_handle.abort();
                                break;
                            }
                        }
                    }
                };
            }
        });
    }
}

impl<T: Service> Drop for ServiceManager<T> {
    fn drop(&mut self) {
        if !self.status.status().is_shutdown() {
            self.controller.send(Control::Kill)
        }
    }
}
