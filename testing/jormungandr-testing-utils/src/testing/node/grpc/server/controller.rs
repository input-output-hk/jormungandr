use super::{MockExitCode, MockLogger, MockServerData, MockVerifier, ProtocolVersion};
use chain_impl_mockchain::{block::Header, key::Hash};
use std::sync::RwLock;
use std::{
    sync::Arc,
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

    pub fn set_tip(&mut self, tip: Header) {
        let mut data = self.data.write().unwrap();
        *data.tip_mut() = tip;
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
