use crate::{
    services::fatal_error,
    settings::{
        start::{Error, RawSettings, Settings},
        CommandLine,
    },
};
use async_trait::async_trait;
use organix::{
    service::Intercom, IntercomMsg, Service, ServiceIdentifier, ServiceState, WatchdogError,
};
use std::{error::Error as _, sync::Arc};
use tokio::sync::{oneshot, watch};

/// Communicate with the configuration service
///
/// see the different _commands_ available
#[derive(IntercomMsg)]
pub struct ConfigApi {
    message: Message,
}

enum Message {
    QuerySettings {
        reply: oneshot::Sender<Arc<Settings>>,
    },
    QueryWatcher {
        reply: oneshot::Sender<watch::Receiver<()>>,
    },
}

/// the Configuration service
///
/// This service is responsible to load the configuration of the node
/// and to keep inform other nodes if the configuration has changed.
///
/// ## TODO
///
/// - [ ] allow dynamic modification of the settings,
/// - [ ] add interface to allow other service to register on settings
///       modifications.
///
pub struct ConfigService {
    state: ServiceState<Self>,
}

impl ConfigApi {
    /// attempt to query the Configuration Service for the settings
    ///
    pub async fn query_settings(
        intercom: &mut Intercom<ConfigService>,
    ) -> Result<Arc<Settings>, WatchdogError> {
        let (reply, receiver) = oneshot::channel();

        let query = ConfigApi {
            message: Message::QuerySettings { reply },
        };

        tracing::debug!("query current settings");
        intercom.send(query).await?;

        match receiver.await {
            Ok(obj) => Ok(obj),
            Err(err) => unreachable!(
                "It appears the ConfigService is up but not responding: {}",
                err
            ),
        }
    }

    /// query a watcher to the configuration a service can be
    /// notified on changes
    pub async fn query_watcher(
        intercom: &mut Intercom<ConfigService>,
    ) -> Result<watch::Receiver<()>, WatchdogError> {
        let (reply, receiver) = oneshot::channel();

        let query = ConfigApi {
            message: Message::QueryWatcher { reply },
        };

        tracing::debug!("query setting watcher");
        intercom.send(query).await?;

        match receiver.await {
            Ok(obj) => Ok(obj),
            Err(err) => unreachable!(
                "It appears the ConfigService is up but not responding: {}",
                err
            ),
        }
    }
}

#[async_trait]
impl Service for ConfigService {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "configuration";

    type IntercomMsg = ConfigApi;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        // load the command line options
        let command_line = CommandLine::load();

        // gentle hack
        if command_line.full_version {
            println!("{}", env!("FULL_VERSION"));
            super::shutdown_with(&mut self.state, 0).await
        } else if command_line.source_version {
            println!("{}", env!("SOURCE_VERSION"));
            super::shutdown_with(&mut self.state, 0).await
        }

        if let Err(error) = start(&mut self.state, command_line).await {
            tracing::error!(source = ?error.source(), "{}", error);
            fatal_error(&mut self.state, error).await;
        }

        tokio::time::delay_for(std::time::Duration::from_secs(10000)).await;
    }
}

async fn start(
    state: &mut ServiceState<ConfigService>,
    command_line: CommandLine,
) -> Result<(), Error> {
    let raw_settings = RawSettings::load(command_line)?;
    let settings = raw_settings.try_into()?;
    let (_sx, rx) = watch::channel(());

    let settings = Arc::new(settings);

    while let Some(ConfigApi { message }) = state.intercom_mut().recv().await {
        match message {
            Message::QueryWatcher { reply } => {
                let rx = rx.clone();
                if let Err(_settings) = reply.send(rx) {
                    // this case should not happen as we control the flow
                    // for awaiting for the reply. So if the settings
                    // is required the other end will wait until it
                    // receives this reply
                    //
                    // Anyhow, we can still ignore the error
                    tracing::debug!("could not reply to the `query_watcher` call");
                }
            }
            Message::QuerySettings { reply } => {
                let settings = Arc::clone(&settings);
                if let Err(_settings) = reply.send(settings) {
                    // this case should not happen as we control the flow
                    // for awaiting for the reply. So if the settings
                    // is required the other end will wait until it
                    // receives this reply
                    //
                    // Anyhow, we can still ignore the error
                    tracing::debug!("could not reply to the `query_settings` call");
                }
            }
        }
    }

    Ok(())
}

impl std::fmt::Debug for ConfigApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigApi").field("reply", &"..").finish()
    }
}
