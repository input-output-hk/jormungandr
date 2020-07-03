use jormungandr_lib::interfaces::notifier::JsonMessage;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use thiserror::Error;
use tungstenite::{connect, Message};
use url::Url;

#[derive(Debug, Error)]
pub enum NotifierError {
    #[error("could not deserialize response")]
    CannotDeserialize(#[from] serde_json::Error),
    #[error("could not send reqeuest")]
    RequestError(#[from] reqwest::Error),
    #[error("hash parse error")]
    HashParseError(#[from] chain_crypto::hash::Error),
}

pub fn uri_from_socket_addr(addr: SocketAddr) -> Url {
    Url::parse(&format!("ws://{}/api/v1/notifier", addr)).unwrap()
}

/// Specialized rest api
#[derive(Debug)]
pub struct JormungandrNotifier {
    url: Url,
    finished: Arc<RwLock<bool>>,
    handles: Vec<JoinHandle<()>>,
}

pub enum NotifierMessage {
    JsonMessage(JsonMessage),
    MaxConnectionsReached,
}

impl JormungandrNotifier {
    pub fn new(url: Url) -> Self {
        JormungandrNotifier {
            url,
            finished: Arc::new(RwLock::new(false)),
            handles: Default::default(),
        }
    }

    pub fn new_client<F>(&mut self, mut for_each: F) -> Result<(), ()>
    where
        F: FnMut(NotifierMessage) -> bool + Send + 'static,
    {
        let url = self.url.clone();
        let (mut socket, _response) = connect(url).expect("Can't connect to notifier websocket");

        // TODO: handle error?

        let finished = Arc::clone(&self.finished);

        let join = std::thread::spawn(move || loop {
            if *finished.read().unwrap() {
                break;
            }

            let msg = socket.read_message().expect("Error reading message");

            match msg {
                Message::Text(text) => {
                    let json_msg: JsonMessage =
                        serde_json::from_str(&text).expect("Deserialization failed");

                    if !for_each(NotifierMessage::JsonMessage(json_msg)) {
                        break;
                    }
                }
                Message::Close(close_frame) => {
                    if let tungstenite::protocol::frame::coding::CloseCode::Library(4000) =
                        close_frame.expect("no close code").code
                    {
                        for_each(NotifierMessage::MaxConnectionsReached);
                    }

                    break;
                }
                _ => unreachable!("unexpected notifier message"),
            }
        });

        self.handles.push(join);

        Ok(())
    }

    pub fn wait_all(&mut self) -> std::thread::Result<()> {
        for handle in self.handles.drain(..) {
            handle.join()?;
        }

        Ok(())
    }
}

impl Drop for JormungandrNotifier {
    fn drop(&mut self) {
        *self.finished.write().unwrap() = true;
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}
