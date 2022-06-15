use super::{MockExitCode, MockLogger, MockServerData, MockVerifier, ProtocolVersion};
use chain_core::property::Serialize;
use chain_impl_mockchain::{
    block::{Block, Header},
    key::Hash,
};
use std::{
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

pub struct MockController {
    verifier: MockVerifier,
    stop_signal: tokio::sync::oneshot::Sender<()>,
    data: Arc<RwLock<MockServerData>>,
    port: u16,
}

impl MockController {
    pub fn new(
        logger: MockLogger,
        stop_signal: tokio::sync::oneshot::Sender<()>,
        data: Arc<RwLock<MockServerData>>,
        port: u16,
    ) -> Self {
        Self {
            verifier: MockVerifier::new(logger),
            stop_signal,
            data,
            port,
        }
    }

    pub fn finish_and_verify_that<F: 'static + std::marker::Send>(
        self,
        verify_func: F,
    ) -> MockExitCode
    where
        F: Fn(&MockVerifier) -> bool,
    {
        let start = Instant::now();
        let timeout = Duration::from_secs(120);

        loop {
            thread::sleep(Duration::from_secs(1));
            if start.elapsed() > timeout {
                self.stop();
                return MockExitCode::Timeout;
            }
            if verify_func(&self.verifier) {
                self.stop();
                return MockExitCode::Success;
            }
        }
    }

    /// block_id must refer to a valid block already in the storage
    pub fn set_tip(&mut self, tip: &Header) {
        let data = self.data.write().unwrap();
        data.set_tip(tip.serialize_as_vec().as_ref().unwrap())
            .unwrap();
    }

    pub fn set_tip_block(&mut self, tip: &Block) {
        let data = self.data.write().unwrap();
        data.put_block(tip).unwrap();
        data.set_tip(tip.header().hash().serialize_as_vec().as_ref().unwrap())
            .unwrap();
    }

    pub fn genesis_hash(&self) -> Hash {
        let data = self.data.read().unwrap();
        *data.genesis_hash()
    }

    pub fn set_genesis(&mut self, tip: Hash) {
        let mut data = self.data.write().unwrap();
        *data.genesis_hash_mut() = tip;
    }

    pub fn set_protocol(&mut self, protocol: ProtocolVersion) {
        let mut data = self.data.write().unwrap();
        *data.protocol_mut() = protocol;
    }

    pub fn stop(self) {
        self.stop_signal.send(()).unwrap();
    }

    pub fn address(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }
}
