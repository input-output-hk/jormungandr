pub mod graphql;
mod handlers;

use futures::Future;
use jormungandr_lib::interfaces::{Cors, Tls};
use std::{net::SocketAddr, time::Duration};
use warp::{http::StatusCode, Filter, Rejection, Reply};

pub fn filter(
    db: crate::db::DB,
    settings: crate::GraphQLSettings,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let with_db = warp::any().map(move || db.clone());
    let with_schema = warp::any().map(graphql::create_schema);
    let with_settings = warp::any().map(move || settings.clone());

    let graphql = warp::path!("graphql")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_db)
        .and(with_schema)
        .and(with_settings)
        .and_then(handlers::graphql)
        .boxed();

    let graphiql = warp::path!("graphiql")
        .and(warp::get())
        .and_then(handlers::graphiql)
        .boxed();

    graphql.or(graphiql).recover(handle_rejection).boxed()
}

/// Convert rejections to actual HTTP errors
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = err.find::<handlers::Error>() {
        let (body, code) = (
            display_internal_server_error(err),
            StatusCode::INTERNAL_SERVER_ERROR,
        );

        return Ok(warp::reply::with_status(body, code));
    }

    Err(err)
}

fn display_internal_server_error(err: &impl std::error::Error) -> String {
    use std::fmt::{self, Write};

    fn error_to_body(err: &impl std::error::Error) -> Result<String, fmt::Error> {
        let mut reply_body = String::new();
        writeln!(reply_body, "Internal server error: {}", err)?;
        let mut source = err.source();
        while let Some(err) = source {
            writeln!(reply_body, "-> {}", err)?;
            source = err.source();
        }
        Ok(reply_body)
    }

    error_to_body(err).unwrap_or_else(|err| format!("failed to process internal error: {}", err))
}

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
