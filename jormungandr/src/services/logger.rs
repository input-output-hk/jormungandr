use crate::services::{ConfigApi, ConfigService};
use async_trait::async_trait;
use organix::{service::NoIntercom, Service, ServiceIdentifier, ServiceState, WatchdogError};
use tokio::sync::watch;
use tracing_futures::*;

/// the logger service, control all the logging mechanism.
///
/// - [ ] add hot reloading/reconfiguration of the settings
/// - [ ] add gelf support
/// - [ ] add syslog support
///
pub struct LoggerService {
    state: ServiceState<Self>,
}

impl LoggerService {
    async fn get_settings_watch(&mut self) -> Result<watch::Receiver<()>, WatchdogError> {
        let mut settings = self.state.intercom_with::<ConfigService>();
        settings.wait_service_started().in_current_span().await?;

        ConfigApi::query_watcher(&mut settings)
            .in_current_span()
            .await
    }
}

#[async_trait]
impl Service for LoggerService {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "logger";

    type IntercomMsg = NoIntercom;

    fn prepare(state: ServiceState<Self>) -> Self {
        let subscriber = tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting tracing default failed");
        Self { state }
    }

    async fn start(mut self) {
        let mut settings = self.state.intercom_with::<ConfigService>();
        settings
            .wait_service_started()
            .in_current_span()
            .await
            .unwrap();

        match self.get_settings_watch().in_current_span().await {
            Err(error) => super::fatal_error(&mut self.state, error).await,
            Ok(mut settings_updated) => {
                while let Some(()) = settings_updated.recv().await {
                    tracing::debug!("settings have been updated...")
                }
            }
        }
    }
}
