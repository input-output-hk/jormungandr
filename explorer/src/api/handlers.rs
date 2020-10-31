use juniper::http::GraphQLRequest;
use thiserror::Error;
use tokio::task::{spawn_blocking, JoinError};
use warp::{reject::Reject, Rejection, Reply};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BlockingError(#[from] JoinError),
}

impl Reject for Error {}

pub async fn graphiql() -> Result<impl Reply, Rejection> {
    let html = juniper::http::graphiql::graphiql_source("/graphql");
    Ok(warp::reply::html(html))
}

pub async fn graphql(
    data: GraphQLRequest,
    db: crate::db::DB,
    schema: super::graphql::Schema,
    settings: super::graphql::GraphQLSettings,
) -> Result<impl Reply, Rejection> {
    let context = super::graphql::Context { db, settings };

    // Run the query in a threadpool, as Juniper is synchronous
    spawn_blocking(move || {
        let response = data.execute(&schema, &context);
        Ok(warp::reply::json(&response))
    })
    .await
    .map_err(Error::BlockingError)
    .map_err(warp::reject::custom)?
}
