use async_trait::async_trait;
use organix::{service::Intercom, IntercomMsg, Service, ServiceIdentifier, ServiceState};

/// console service, control standard output displays
/// (including the logs)
pub struct ConsoleService {
    state: ServiceState<Self>,
}

#[derive(Debug, IntercomMsg)]
pub struct ConsoleApi {
    message: Message,
}

#[derive(Debug)]
enum Message {
    Error {
        error: Box<dyn std::error::Error + Send>,
    },
}

impl ConsoleApi {
    /// forward an error to the console for display
    ///
    /// if the console cannot take in the error object, the error will
    /// be simplify dismissed.
    pub fn error<E>(intercom: &mut Intercom<ConsoleService>, error: E)
    where
        E: std::error::Error + Send + 'static,
    {
        let message = Message::Error {
            error: Box::new(error),
        };

        if let Err(error) = intercom.try_send(Self { message }) {
            tracing::error!(%error, "could not send error to the console")
        }
    }
}

#[async_trait]
impl Service for ConsoleService {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "console";

    type IntercomMsg = ConsoleApi;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        let recv = self.state.intercom_mut();

        while let Some(ConsoleApi { message }) = recv.recv().await {
            match message {
                Message::Error { error } => {
                    eprintln!("{}", error);
                }
            }
        }

        // todo, monitor statuses? Heart beat?
    }
}
