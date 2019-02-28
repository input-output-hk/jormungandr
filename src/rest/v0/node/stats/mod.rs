mod stats_counter;

pub use self::stats_counter::StatsCounter;

use actix_web::middleware::cors::Cors;
use actix_web::{App, Json, Responder, State};

pub fn crate_handler(
    stats_counter: StatsCounter,
) -> impl Fn(&str) -> App<StatsCounter> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        let app_prefix = format!("{}/v0/node/stats", prefix);
        App::with_state(stats_counter.clone())
            .prefix(app_prefix)
            .resource("", |r| r.get().with(handle_request))
    }
}

fn handle_request(stats: State<StatsCounter>) -> impl Responder {
    Json(json!({
        "txRecvCnt": stats.get_tx_recv_cnt(),
        "blockRecvCnt": stats.get_block_recv_cnt(),
        "uptime": stats.get_uptime_sec(),
    }))
}
