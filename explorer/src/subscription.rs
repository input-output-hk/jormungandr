use async_tungstenite::{
    tungstenite::protocol::frame::coding::CloseCode, tungstenite::protocol::Message,
};
use futures_util::{StreamExt, TryStreamExt};
use jormungandr_lib::interfaces::notifier;
use slog::info;
use thiserror::Error;

pub use notifier::JsonMessage;

#[derive(Debug, Error)]
pub enum SubscriptionError {
    #[error("max connections reached")]
    MaxConnectionsReached,
    #[error("unexpected close event")]
    UnexpectedCloseEvent,
    #[error(transparent)]
    Tungstenite(#[from] async_tungstenite::tungstenite::Error),
    #[error(transparent)]
    CannotDeserialize(#[from] serde_json::Error),
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
}

pub async fn start_subscription(
    url: url::Url,
    logger: slog::Logger,
) -> Result<impl StreamExt<Item = Result<JsonMessage, SubscriptionError>>, SubscriptionError> {
    info!(logger, "starting subscription");
    let (ws_stream, _) = async_tungstenite::tokio::connect_async(url).await?;
    info!(
        logger,
        "WebSocket handshake has been successfully completed"
    );

    let (_write, read) = ws_stream.split();

    Ok(read
        .map_err(SubscriptionError::Tungstenite)
        .and_then(process_message))
}

async fn process_message(msg: Message) -> Result<JsonMessage, SubscriptionError> {
    match msg {
        Message::Text(text) => {
            let json_msg: JsonMessage = serde_json::from_str(&text)?;
            Ok(json_msg)
        }
        Message::Close(close_frame) => match close_frame.expect("no close code").code {
            CloseCode::Library(4000) => Err(SubscriptionError::MaxConnectionsReached),
            _ => Err(SubscriptionError::UnexpectedCloseEvent),
        },
        _ => unreachable!("unexpected notifier message"),
    }
}
