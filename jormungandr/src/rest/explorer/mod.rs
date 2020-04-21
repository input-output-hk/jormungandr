mod handlers;

use crate::rest::{display_internal_server_error, ContextLock};

use warp::{http::StatusCode, Filter, Rejection, Reply};

pub fn filter(
    context: ContextLock,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let with_context = warp::any().map(move || context.clone());
    let root = warp::path!("explorer" / ..);

    let graphql = warp::path!("graphql")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context)
        .and_then(handlers::graphql)
        .boxed();

    let graphiql = warp::path!("graphiql")
        .and(warp::get())
        .and_then(handlers::graphiql)
        .boxed();

    root.and(graphql.or(graphiql))
        .recover(handle_rejection)
        .boxed()
}

/// Convert rejections to actual HTTP errors
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = err.find::<handlers::Error>() {
        let (body, code) = match err {
            handlers::Error::ProcessingError => (err.to_string(), StatusCode::BAD_REQUEST),
            err => (
                display_internal_server_error(err),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        };

        return Ok(warp::reply::with_status(body, code));
    }

    Err(err)
}
