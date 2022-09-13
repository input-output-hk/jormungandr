pub mod graphql;

use self::graphql::EContext;
use crate::db::ExplorerDb;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use futures::Future;
use jormungandr_lib::interfaces::{Cors, Tls};
use std::{net::SocketAddr, time::Duration};
use warp::{http::Response as HttpResponse, Filter, Rejection, Reply};

pub async fn setup_cors<API>(
    api: API,
    listen_addr: SocketAddr,
    tls_config: Option<Tls>,
    cors_config: Option<Cors>,
    shutdown_signal: impl Future<Output = ()> + Send + 'static,
) where
    API: Filter<Error = warp::Rejection> + Clone + Send + Sync + 'static,
    API::Extract: warp::Reply,
{
    match cors_config {
        Some(config) => {
            let allowed_origins: Vec<&str> =
                config.allowed_origins.iter().map(AsRef::as_ref).collect();

            let mut cors = warp::cors().allow_origins(allowed_origins);

            if let Some(max_age) = config.max_age_secs {
                cors = cors.max_age(Duration::from_secs(max_age));
            }

            serve(api.with(cors), listen_addr, tls_config, shutdown_signal).await;
        }
        None => serve(api, listen_addr, tls_config, shutdown_signal).await,
    }

    tracing::info!("listening on: {}", listen_addr);
}

async fn serve<API>(
    api: API,
    listen_addr: SocketAddr,
    tls_config: Option<Tls>,
    shutdown_signal: impl Future<Output = ()> + Send + 'static,
) where
    API: Filter<Error = warp::Rejection> + Clone + Send + Sync + 'static,
    API::Extract: warp::Reply,
{
    let server = warp::serve(api);
    match tls_config {
        Some(tls_config) => {
            let (_, server_fut) = server
                .tls()
                .cert_path(tls_config.cert_file)
                .key_path(tls_config.priv_key_file)
                .bind_with_graceful_shutdown(listen_addr, shutdown_signal);
            server_fut.await;
        }
        None => {
            let (_, server_fut) = server.bind_with_graceful_shutdown(listen_addr, shutdown_signal);
            server_fut.await;
        }
    }
}

pub fn filter(
    db: ExplorerDb,
    settings: crate::db::Settings,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let schema = async_graphql::Schema::build(
        crate::api::graphql::Query {},
        async_graphql::EmptyMutation,
        crate::api::graphql::Subscription {},
    )
    .limit_depth(settings.query_depth_limit)
    .limit_complexity(settings.query_complexity_limit)
    .data(EContext { db, settings })
    .finish();

    let graphql_post = async_graphql_warp::graphql(schema.clone())
        .and_then(|(schema, request)| handler(schema, request));

    let graphql = warp::path!("graphql").and(graphql_post).boxed();

    let graphql_playground = warp::path::end().and(warp::get()).map(|| {
        HttpResponse::builder()
            .header("content-type", "text/html")
            .body(playground_source(
                GraphQLPlaygroundConfig::new("/graphql").subscription_endpoint("/subscription"),
            ))
    });

    let subscription =
        warp::path!("subscription").and(async_graphql_warp::graphql_subscription(schema));

    let playground = warp::path!("playground").and(graphql_playground).boxed();

    subscription
        .or(graphql)
        .or(playground)
        .with(warp::filters::trace::trace(|info| {
            use http_zipkin::get_trace_context;
            use tracing::field::Empty;
            let span = tracing::span!(
                tracing::Level::DEBUG,
                "rest_api_request",
                method = %info.method(),
                path = info.path(),
                version = ?info.version(),
                remote_addr = Empty,
                trace_id = Empty,
                span_id = Empty,
                parent_span_id = Empty,
            );
            if let Some(remote_addr) = info.remote_addr() {
                span.record("remote_addr", remote_addr.to_string().as_str());
            }
            if let Some(trace_context) = get_trace_context(info.request_headers()) {
                span.record("trace_id", trace_context.trace_id().to_string().as_str());
                span.record("span_id", trace_context.span_id().to_string().as_str());
                if let Some(parent_span_id) = trace_context.parent_id() {
                    span.record("parent_span_id", parent_span_id.to_string().as_str());
                }
            }
            span
        }))
}

pub async fn handler(
    schema: graphql::Schema,
    request: async_graphql::Request,
) -> Result<impl Reply, std::convert::Infallible> {
    Ok::<_, std::convert::Infallible>(async_graphql_warp::GraphQLResponse::from(
        schema.execute(request).await,
    ))
}
