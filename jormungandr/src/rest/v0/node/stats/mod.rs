use crate::stats_counter::StatsCounter;

use actix_web::App;
use crate::rest::v0::handlers;

pub fn create_handler(
    stats_counter: StatsCounter,
) -> impl Fn(&str) -> App<StatsCounter> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        let app_prefix = format!("{}/v0/node/stats", prefix);
        App::with_state(stats_counter.clone())
            .prefix(app_prefix)
            .resource("", |r| r.get().with(handlers::get_stats_counter))
    }
}
