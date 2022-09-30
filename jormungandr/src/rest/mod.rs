//! REST API of the node
#[cfg(feature = "prometheus-metrics")]
mod prometheus;
pub mod v0;
mod v1;

use crate::context::{Context, ContextLock, ServerStopper};
use futures::{channel::mpsc, prelude::*};
use jormungandr_lib::interfaces::{Cors, Tls};
use std::{error::Error, net::SocketAddr, time::Duration};
use warp::Filter;

pub struct Config {
    pub listen: SocketAddr,
    pub tls: Option<Tls>,
    pub cors: Option<Cors>,
    #[cfg(feature = "prometheus-metrics")]
    pub enable_prometheus: bool,
}

pub async fn start_rest_server(config: Config, context: ContextLock) {
    let (stopper_tx, stopper_rx) = mpsc::channel::<()>(0);
    let stopper_rx = stopper_rx.into_future().map(|_| ());
    context
        .write()
        .await
        .set_rest_server_stopper(ServerStopper::new(stopper_tx));
    let api = v0::filter(context.clone()).or(v1::filter(context.clone()));

    let api = warp::path!("api" / ..)
        .and(api)
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
        }));

    setup_prometheus(api, config, context, stopper_rx).await;
}

#[cfg(feature = "prometheus-metrics")]
async fn setup_prometheus<App>(
    app: App,
    config: Config,
    context: ContextLock,
    shutdown_signal: impl Future<Output = ()> + Send + 'static,
) where
    App: Filter<Error = warp::Rejection> + Clone + Send + Sync + 'static,
    App::Extract: warp::Reply,
{
    if config.enable_prometheus {
        let prometheus = prometheus::filter(context.clone());
        setup_cors(app.or(prometheus), config, shutdown_signal).await;
    } else {
        setup_cors(app, config, shutdown_signal).await;
    }
}

#[cfg(not(feature = "prometheus-metrics"))]
async fn setup_prometheus<App>(
    app: App,
    config: Config,
    _context: ContextLock,
    shutdown_signal: impl Future<Output = ()> + Send + 'static,
) where
    App: Filter<Error = warp::Rejection> + Clone + Send + Sync + 'static,
    App::Extract: warp::Reply,
{
    setup_cors(app, config, shutdown_signal).await;
}

async fn setup_cors<App>(
    app: App,
    config: Config,
    shutdown_signal: impl Future<Output = ()> + Send + 'static,
) where
    App: Filter<Error = warp::Rejection> + Clone + Send + Sync + 'static,
    App::Extract: warp::Reply,
{
    if let Some(cors_config) = config.cors {
        let allowed_origins: Vec<&str> = cors_config
            .allowed_origins
            .iter()
            .map(AsRef::as_ref)
            .collect();

        let mut cors = warp::cors().allow_origins(allowed_origins);

        for header in cors_config.allowed_headers {
            cors = cors.allow_header(header.0);
        }

        for method in cors_config.allowed_methods {
            cors = cors.allow_method(method.0);
        }

        if let Some(max_age) = cors_config.max_age_secs {
            cors = cors.max_age(Duration::from_secs(max_age));
        }

        run_server_with_app(
            app.with(cors.build()),
            config.listen,
            config.tls,
            shutdown_signal,
        )
        .await;
    } else {
        run_server_with_app(app, config.listen, config.tls, shutdown_signal).await;
    }
}

async fn run_server_with_app<App>(
    app: App,
    listen_addr: SocketAddr,
    tls_config: Option<Tls>,
    shutdown_signal: impl Future<Output = ()> + Send + 'static,
) where
    App: Filter<Error = warp::Rejection> + Clone + Send + Sync + 'static,
    App::Extract: warp::Reply,
{
    let server = warp::serve(app);
    if let Some(tls_config) = tls_config {
        let (_, server_fut) = server
            .tls()
            .cert_path(tls_config.cert_file)
            .key_path(tls_config.priv_key_file)
            .bind_with_graceful_shutdown(listen_addr, shutdown_signal);
        server_fut.await;
    } else {
        let (_, server_fut) = server.bind_with_graceful_shutdown(listen_addr, shutdown_signal);
        server_fut.await;
    };
}

pub(self) fn display_internal_server_error(err: &impl Error) -> String {
    use std::fmt::{self, Write};

    fn error_to_body(err: &impl Error) -> Result<String, fmt::Error> {
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
