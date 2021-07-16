mod handlers;
mod logic;

use crate::rest::{display_internal_server_error, ContextLock};

use jormungandr_lib::interfaces::{Address, VotePlanId};

use warp::{http::StatusCode, Filter, Rejection, Reply};

pub fn filter(
    context: ContextLock,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let with_context = warp::any().map(move || context.clone());
    let root = warp::path!("v1" / ..);

    let fragments = {
        let root = warp::path!("fragments" / ..);

        let post = warp::path::end()
            .and(warp::post())
            .and(warp::body::json())
            .and(with_context.clone())
            .and_then(handlers::post_fragments)
            .boxed();

        let status = warp::path!("statuses")
            .and(warp::get())
            .and(warp::query())
            .and(with_context.clone())
            .and_then(handlers::get_fragment_statuses)
            .boxed();

        let logs = warp::path!("logs")
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_fragment_logs)
            .boxed();

        root.and(post.or(status).or(logs)).boxed()
    };

    let votes = warp::path!("votes" / "plan" / VotePlanId / "account-votes" / Address)
        .and(warp::get())
        .and(with_context)
        .and_then(handlers::get_account_votes);

    let routes = fragments;

    root.and(routes.or(votes)).recover(handle_rejection).boxed()
}

/// Convert rejections to actual HTTP errors
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = err.find::<logic::Error>() {
        let (body, code) = match err {
            logic::Error::PublicKey(_) | logic::Error::Hash(_) | logic::Error::Hex(_) => {
                (err.to_string(), StatusCode::BAD_REQUEST)
            }
            logic::Error::Fragments(summary) => (
                serde_json::to_string(&summary).unwrap(),
                StatusCode::BAD_REQUEST,
            ),
            err => (
                display_internal_server_error(err),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        };

        return Ok(warp::reply::with_status(body, code));
    }

    Err(err)
}
