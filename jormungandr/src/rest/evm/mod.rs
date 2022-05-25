use crate::context::ContextLock;
use reqwest::StatusCode;
use warp::{Filter, Rejection, Reply};

use super::display_internal_server_error;

mod handlers;
mod logic;

pub fn filter(
    context: ContextLock,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let with_context = warp::any().map(move || context.clone());

    let root = warp::path!("v2" / ..);

    let address_mapping = {
        let root = warp::path!("address_mapping" / ..);

        let get_jor_address = warp::path!("jormungandr_address" / String)
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_jor_address)
            .boxed();

        let get_evm_address = warp::path!("evm_address" / String)
            .and(warp::get())
            .and(with_context)
            .and_then(handlers::get_evm_address)
            .boxed();

        root.and(get_jor_address.or(get_evm_address)).boxed()
    };

    let routes = address_mapping;

    root.and(routes).recover(handle_rejection).boxed()
}

/// Convert rejections to actual HTTP errors
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = err.find::<logic::Error>() {
        let (body, code) = (
            display_internal_server_error(err),
            StatusCode::INTERNAL_SERVER_ERROR,
        );

        return Ok(warp::reply::with_status(body, code));
    }

    Err(err)
}
