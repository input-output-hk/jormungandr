//! REST API of the node

pub mod context;
pub mod explorer;
pub mod v0;

pub use self::context::{Context, FullContext};

use crate::settings::start::{Rest, Tls};

use futures03::{channel::mpsc, prelude::*};
use std::{net::SocketAddr, time::Duration};
use warp::Filter;

#[derive(Clone)]
pub struct ServerStopper(mpsc::Sender<()>);

impl ServerStopper {
    pub fn stop(&mut self) {
        self.0.try_send(()).unwrap();
    }
}

pub async fn start_rest_server(config: Rest, explorer_enabled: bool, context: Context) {
    let (stopper_tx, stopper_rx) = mpsc::channel::<()>(0);
    let stopper_rx = stopper_rx.into_future().map(|_| ());
    context.set_server_stopper(ServerStopper(stopper_tx)).await;

    let api = v0::filter(context.clone());
    if explorer_enabled {
        let explorer = explorer::filter(context);
        setup_cors(api.or(explorer), config, stopper_rx).await;
    } else {
        setup_cors(api, config, stopper_rx).await;
    }
}

async fn setup_cors<App>(
    app: App,
    config: Rest,
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

        if let Some(max_age) = cors_config.max_age_secs {
            cors = cors.max_age(Duration::from_secs(max_age));
        }

        run_server_with_app(app.with(cors), config.listen, config.tls, shutdown_signal).await;
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
