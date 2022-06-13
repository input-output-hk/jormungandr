use crate::rest::ContextLock;
use warp::{Filter, Rejection, Reply};

pub fn filter(
    context: ContextLock,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("prometheus")
        .and(warp::get())
        .and(warp::any().map(move || context.clone()))
        .and_then(|context: ContextLock| async move {
            let context = context.read().await;
            let full_context = context.try_full().map_err(warp::reject::custom)?;
            full_context
                .prometheus
                .as_ref()
                .expect("Prometheus metrics exporter not set in API context!")
                .http_response()
        })
}
