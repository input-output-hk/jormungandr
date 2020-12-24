use crate::rest::{context, display_internal_server_error, ContextLock};
use thiserror::Error;
use warp::reject::Reject;
use warp::{http::StatusCode, Filter, Rejection, Reply};

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum ExplorerGraphQLError {
    #[error(transparent)]
    Context(#[from] context::Error),
}

impl Reject for ExplorerGraphQLError {}

pub fn filter(
    context: ContextLock,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let with_context = warp::any().map(move || context.clone());
    let root = warp::path!("explorer" / ..);

    let context_extractor = with_context
        .and_then(|context: ContextLock| async move {
            context
                .read()
                .await
                .try_full()
                .map_err(ExplorerGraphQLError::Context)
                .map_err(warp::reject::custom)
                .map(|ctx| ctx.explorer.clone().unwrap().context())
        })
        .boxed();

    let graphql_filter =
        juniper_warp::make_graphql_filter(crate::explorer::create_schema(), context_extractor);

    let graphql = warp::path!("graphql").and(graphql_filter).boxed();

    let graphiql_filter = juniper_warp::graphiql_filter("/explorer/graphql", None);

    let graphiql = warp::path!("graphiql").and(graphiql_filter).boxed();

    root.and(graphql.or(graphiql))
        .recover(handle_rejection)
        .boxed()
}

/// Convert rejections to actual HTTP errors
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = err.find::<ExplorerGraphQLError>() {
        let (body, code) = (
            display_internal_server_error(err),
            StatusCode::INTERNAL_SERVER_ERROR,
        );

        return Ok(warp::reply::with_status(body, code));
    }

    Err(err)
}
