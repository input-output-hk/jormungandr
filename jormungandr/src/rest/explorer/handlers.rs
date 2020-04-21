use crate::{
    explorer::graphql::GraphQLRequest,
    rest::{context, ContextLock},
};

use thiserror::Error;
use tokio02::task::{spawn_blocking, JoinError};
use warp::{reject::Reject, Rejection, Reply};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Context(#[from] context::Error),
    #[error("Error processing query")]
    ProcessingError,
    #[error(transparent)]
    BlockingError(#[from] JoinError),
}

impl Reject for Error {}

pub async fn graphiql() -> Result<impl Reply, Rejection> {
    let html = juniper::http::graphiql::graphiql_source("/explorer/graphql");
    Ok(warp::reply::html(html))
}

pub async fn graphql(data: GraphQLRequest, context: ContextLock) -> Result<impl Reply, Rejection> {
    let explorer = context
        .read()
        .await
        .try_full()
        .map_err(Error::Context)
        .map_err(warp::reject::custom)?
        .explorer
        .clone()
        .unwrap();

    // Run the query in a threadpool, as Juniper is synchronous
    spawn_blocking(move || {
        let response = data.execute(&explorer.schema, &explorer.context());
        if response.is_ok() {
            Ok(warp::reply::json(&response))
        } else {
            Err(warp::reject::custom(Error::ProcessingError))
        }
    })
    .await
    .map_err(Error::BlockingError)
    .map_err(warp::reject::custom)?
}
