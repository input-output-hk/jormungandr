use crate::rest::{context, ContextLock};
use thiserror::Error;
use warp::{reject::Reject, Filter, Rejection, Reply};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Context(#[from] context::Error),
}

impl Reject for Error {}

pub fn filter(
    context: ContextLock,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let with_context = warp::any().map(move || context.clone());

    let notifier = warp::path!("notifier")
        .and(warp::ws())
        .and(with_context)
        .and_then(handle_connection);

    notifier.boxed()
}

async fn handle_connection(
    ws: warp::ws::Ws,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    let full_context = context
        .try_full()
        .map_err(Error::Context)
        .map_err(warp::reject::custom)?;

    let notifier: crate::notifier::Notifier = full_context.notifier.clone();

    Ok(ws.on_upgrade(move |socket| add_connection(notifier, socket)))
}

async fn add_connection(notifier: crate::notifier::Notifier, socket: warp::ws::WebSocket) {
    notifier.new_connection(socket).await;
}
