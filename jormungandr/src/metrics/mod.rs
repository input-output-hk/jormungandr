use crate::blockchain::Ref;
use chain_impl_mockchain::block::Block;
use jormungandr_lib::time::SecondsSinceUnixEpoch;
use std::sync::Arc;

pub mod backends;

pub trait MetricsBackend {
    fn add_tx_recv_cnt(&self, count: usize);
    fn set_mempool_usage_ratio(&self, ratio: f64);
    fn set_mempool_total_size(&self, size: usize);
    fn add_tx_rejected_cnt(&self, count: usize);
    fn add_block_recv_cnt(&self, count: usize);
    fn add_peer_connected_cnt(&self, count: usize);
    fn sub_peer_connected_cnt(&self, count: usize);
    fn add_peer_quarantined_cnt(&self, count: usize);
    fn sub_peer_quarantined_cnt(&self, count: usize);
    fn set_peer_available_cnt(&self, count: usize);
    fn set_slot_start_time(&self, time: SecondsSinceUnixEpoch);
    fn set_tip_block(&self, block: &Block, block_ref: &Ref);
}

#[derive(Clone)]
pub struct Metrics {
    backends: Vec<Arc<dyn MetricsBackend + Send + Sync + 'static>>,
}

impl Metrics {
    pub fn builder() -> MetricsBuilder {
        MetricsBuilder::default()
    }
}

#[derive(Default)]
pub struct MetricsBuilder {
    backends: Vec<Arc<dyn MetricsBackend + Send + Sync + 'static>>,
}

impl MetricsBuilder {
    pub fn add_backend(mut self, backend: Arc<dyn MetricsBackend + Send + Sync + 'static>) -> Self {
        self.backends.push(backend);
        self
    }

    pub fn build(self) -> Metrics {
        Metrics {
            backends: self.backends,
        }
    }
}

macro_rules! metrics_method {
    ($name: ident, $type: ident) => {
        fn $name(&self, input: $type) {
            for backend in &self.backends {
                backend.$name(input);
            }
        }
    };
}

macro_rules! metrics_count_method {
    ($name: ident) => {
        metrics_method!($name, usize);
    };
}

impl MetricsBackend for Metrics {
    metrics_count_method!(add_tx_recv_cnt);
    metrics_count_method!(add_tx_rejected_cnt);
    metrics_method!(set_mempool_usage_ratio, f64);
    metrics_count_method!(set_mempool_total_size);
    metrics_count_method!(add_block_recv_cnt);
    metrics_count_method!(add_peer_connected_cnt);
    metrics_count_method!(sub_peer_connected_cnt);
    metrics_count_method!(add_peer_quarantined_cnt);
    metrics_count_method!(sub_peer_quarantined_cnt);
    metrics_count_method!(set_peer_available_cnt);
    metrics_method!(set_slot_start_time, SecondsSinceUnixEpoch);

    fn set_tip_block(&self, block: &Block, block_ref: &Ref) {
        for backend in &self.backends {
            backend.set_tip_block(block, block_ref);
        }
    }
}
