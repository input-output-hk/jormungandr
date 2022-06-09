use crate::jormungandr::JormungandrLogger;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    io::{Read, Result},
    sync::mpsc::Receiver,
};

pub struct MockLogger(JormungandrLogger);

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub enum MethodType {
    Init,
    Handshake,
    ClientAuth,
    PullBlocks,
    PullBlocksToTip,
    Tip,
    GetBlocks,
    GetHeaders,
    GetFragments,
    GetPeers,
    PullHeaders,
    PushHeaders,
    UploadBlocks,
    BlockSubscription,
    FragmentSubscription,
    GossipSubscription,
}

impl fmt::Display for MethodType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

struct ChannelReader(Receiver<Vec<u8>>);

impl Read for ChannelReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self.0.recv() {
            Ok(vec) => vec.as_slice().read(buf),
            // It can only fail if the sending half is disconnected,
            // implying that no further messages will ever be received.
            Err(_) => Ok(0),
        }
    }
}

impl MockLogger {
    pub fn new(rx: Receiver<Vec<u8>>) -> Self {
        let panic_channel = ChannelReader(std::sync::mpsc::channel().1);
        Self(JormungandrLogger::new(ChannelReader(rx), panic_channel))
    }

    pub fn get_log_content(&self) -> String {
        self.0.get_log_content()
    }

    pub fn executed_at_least_once(&self, method: MethodType) -> bool {
        let expected = method.to_string();
        self.0
            .get_lines()
            .iter()
            .any(|entry| entry.fields.get("method") == Some(&expected))
    }
}
