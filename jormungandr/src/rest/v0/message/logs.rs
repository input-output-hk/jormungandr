use crate::fragment::Logs;
use actix_web::{App, Json, Responder, State};
use futures::Future;
use std::sync::{Arc, Mutex};

pub fn create_handler(
    logs: Arc<Mutex<Logs>>,
) -> impl Fn(&str) -> App<Arc<Mutex<Logs>>> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        let app_prefix = format!("{}/v0/fragment/logs", prefix);
        App::with_state(logs.clone())
            .prefix(app_prefix)
            .resource("", |r| r.get().with(handle_request))
    }
}

fn handle_request(logs: State<Arc<Mutex<Logs>>>) -> impl Responder {
    let logs = logs.lock().unwrap();
    let logs = logs.logs().wait().unwrap();
    Json(logs)
}
