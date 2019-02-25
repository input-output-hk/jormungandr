mod stats_counter;

pub use self::stats_counter::StatsCounter;

use actix_web::{App, Json, Responder, State};

pub fn crate_handler(stats_counter: StatsCounter) -> impl Fn() -> App<StatsCounter> + Send + Sync + Clone + 'static {
    move || {
        App::with_state(stats_counter.clone())
            .prefix("api")
            .scope("v0", |scope| {
                scope.resource("v0/node/stats", |r| r.get().with(handle_request))
            })
    }
}

fn handle_request(stats: State<StatsCounter>) -> impl Responder {
    Json(json!({
        "txRecvCnt": stats.get_tx_recv_cnt(),
        "blockRecvCnt": stats.get_block_recv_cnt(),
        "uptime": stats.get_uptime_sec(),
    }))
}
