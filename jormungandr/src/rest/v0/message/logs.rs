use crate::fragment::Logs;
use actix_web::App;
use std::sync::{Arc, Mutex};
use crate::rest::v0::handlers;

pub fn create_handler(
    logs: Arc<Mutex<Logs>>,
) -> impl Fn(&str) -> App<Arc<Mutex<Logs>>> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        let app_prefix = format!("{}/v0/fragment/logs", prefix);
        App::with_state(logs.clone())
            .prefix(app_prefix)
            .resource("", |r| r.get().with(handlers::get_message_logs))
    }
}