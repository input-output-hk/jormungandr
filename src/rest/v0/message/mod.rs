use crate::intercom::TransactionMsg;
use crate::utils::task::TaskMessageBox;
use actix_web::error::ErrorBadRequest;
use actix_web::{App, Error as ActixError, HttpMessage, HttpRequest, Responder};
use bytes::IntoBuf;
use chain_core::property::Deserialize;
use chain_impl_mockchain::message::Message;
use futures::Future;
use std::sync::{Arc, Mutex};

pub type Task = Arc<Mutex<TaskMessageBox<TransactionMsg>>>;

pub fn create_handler(
    transaction_task: Task,
) -> impl Fn(&str) -> App<Task> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        let app_prefix = format!("{}/v0/message", prefix);
        App::with_state(transaction_task.clone())
            .prefix(app_prefix)
            .resource("", |r| r.post().a(handle_request))
    }
}

fn handle_request(
    request: &HttpRequest<Task>,
) -> impl Future<Item = impl Responder + 'static, Error = impl Into<ActixError> + 'static> + 'static
{
    let sender = request.state().clone();
    request.body().map(move |message| -> Result<_, ActixError> {
        println!("qquququq {:?}", message);
        let msg = Message::deserialize(message.into_buf()).map_err(|e| { println!("{}", e); ErrorBadRequest(e) })?;
        let msg = TransactionMsg::SendTransaction(vec![msg]);
        sender.lock().unwrap().send_to(msg);
        Ok("")
    })
}
