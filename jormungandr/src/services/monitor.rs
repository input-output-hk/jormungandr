use crate::services::{ConfigService, ConsoleService, LoggerService};
use async_trait::async_trait;
use organix::{
    service::{NoIntercom, Status},
    Service, ServiceIdentifier, ServiceState, WatchdogError, WatchdogQuery,
};
use tracing_futures::Instrument as _;

/// the Monitoring service
///
/// This service is responsible to start and keep all the different service
/// up as needed.
///
pub struct MonitorService {
    state: ServiceState<Self>,
}

impl MonitorService {
    async fn boot(&mut self) -> Result<(), WatchdogError> {
        let mut watchdog = self.state.watchdog_controller().clone();

        self.start::<LoggerService>(&mut watchdog)
            .in_current_span()
            .await?;

        self.start::<ConsoleService>(&mut watchdog)
            .in_current_span()
            .await?;
        self.start::<ConfigService>(&mut watchdog)
            .in_current_span()
            .await?;

        Ok(())
    }

    async fn start<T: Service>(
        &mut self,
        watchdog: &mut WatchdogQuery,
    ) -> Result<(), WatchdogError> {
        let mut number_attempts = 0;
        const MAX_NUMBER_ATTEMPTS: usize = 2;

        loop {
            tracing::debug!(
                attempt = number_attempts,
                service = T::SERVICE_IDENTIFIER,
                "starting"
            );
            let status = watchdog.status::<T>().await?;

            if number_attempts >= MAX_NUMBER_ATTEMPTS {
                return Err(WatchdogError::CannotStartService {
                    service_identifier: T::SERVICE_IDENTIFIER,
                    source: organix::service::ServiceError::CannotStart {
                        status: status.status,
                    },
                });
            } else {
                number_attempts += 1;
            }
            match status.status {
                Status::Shutdown { .. } => {
                    watchdog.start::<T>().await?;
                }
                Status::ShuttingDown { .. } => {
                    // wait
                    tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
                }
                Status::Starting { .. } => {
                    // wait
                    tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
                }
                Status::Started { .. } => {
                    break;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Service for MonitorService {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "monitoring";

    type IntercomMsg = NoIntercom;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        if let Err(error) = self.boot().in_current_span().await {
            super::fatal_error(&mut self.state, error).await
        }

        // todo, monitor statuses?
    }
}
