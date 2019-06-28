use crate::intercom::TransactionMsg;
use crate::rest::v0::handlers;
use crate::utils::async_msg::MessageBox;

use actix_web::App;

use std::sync::{Arc, Mutex};

pub type Task = Arc<Mutex<MessageBox<TransactionMsg>>>;

pub fn create_handler(
    transaction_task: Task,
) -> impl Fn(&str) -> App<Task> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        let app_prefix = format!("{}/v0/message", prefix);
        App::with_state(transaction_task.clone())
            .prefix(app_prefix)
            .resource("", |r| r.post().a(handlers::post_message))
    }
}
