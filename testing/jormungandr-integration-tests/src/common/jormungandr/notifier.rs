use jormungandr_lib::crypto::hash::Hash;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use thiserror::Error;
use tungstenite::connect;
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
    Url::parse(&format!("ws://{}/notifier", addr)).unwrap()
}

/// Specialized rest api
#[derive(Debug)]
pub struct JormungandrNotifier {
    url: Url,
    finished: Arc<RwLock<bool>>,
    handles: Vec<JoinHandle<()>>,
}

// TODO: maybe this can be shared with the type in jormungandr (that only implements Serialize)
#[derive(Deserialize, Debug)]
pub enum JsonMessage {
    NewBlock(Hash),
    NewTip(Hash),
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
        F: FnMut(JsonMessage) -> () + Send + 'static,
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

            let json_msg = serde_json::from_str(msg.to_text().expect("message is not text"))
                .expect("Deserialization failed");

            for_each(json_msg);
        });

        self.handles.push(join);

        Ok(())
    }
}

impl Drop for JormungandrNotifier {
    fn drop(&mut self) {
        *self.finished.write().unwrap() = true;
        for handle in self.handles.drain(..) {
            handle.join().expect("failed to join thread");
        }
    }
}
