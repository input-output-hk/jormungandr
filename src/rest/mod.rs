//! REST API of the node

mod server_service;
mod server_state;

pub use self::server_service::{Error, ServerService};
pub use self::server_state::ServerState;

use actix_web::{App, Json, Responder, State};
use settings::{Error as ConfigError, Rest};

pub fn start_rest_server(config: &Rest, state: ServerState) -> Result<ServerService, ConfigError> {
    let handler = move || {
        App::with_state(state.clone())
            .prefix("api")
            .scope("v1", |scope| {
                scope.resource("/node-info", |r| r.get().with(node_info_v1))
            })
    };
    ServerService::start(&config.pkcs12, config.listen.clone(), handler)
        .map_err(|e| ConfigError::InvalidRest(e))
}

fn node_info_v1(state: State<ServerState>) -> impl Responder {
    Json(json!({
      "data": {
        "txRecvCnt": state.stats.get_tx_recv_cnt(),
        "blockRecvCnt": state.stats.get_block_recv_cnt(),
      },
      "status": "success"
    }))
}
