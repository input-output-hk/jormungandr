mod handlers;
mod logic;

use crate::rest::{display_internal_server_error, ContextLock};
use jormungandr_lib::interfaces::VotePlanId;
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

    let votes_with_plan = warp::path!("votes" / "plan" / VotePlanId / "account-votes" / String)
        .and(warp::get())
        .and(with_context.clone())
        .and_then(handlers::get_account_votes_with_plan);

    let votes = warp::path!("votes" / "plan" / "account-votes" / String)
        .and(warp::get())
        .and(with_context.clone())
        .and_then(handlers::get_account_votes);

    let votes_count = warp::path!("votes" / "plan" / "accounts-votes-all")
        .and(warp::get())
        .and(with_context)
        .and_then(handlers::get_accounts_votes_all);

    let routes = fragments.or(votes_with_plan).or(votes).or(votes_count);

    root.and(routes).recover(handle_rejection).boxed()
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
