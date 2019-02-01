use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct SharedStats(Arc<Stats>);

impl Deref for SharedStats {
    type Target = Stats;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Default)]
pub struct Stats {
    tx_recv_cnt: AtomicUsize,
    block_recv_cnt: AtomicUsize,
}

impl Stats {
    pub fn add_tx_recv_cnt(&self, count: usize) {
        self.tx_recv_cnt.fetch_add(count, Ordering::Relaxed);
    }

    pub fn get_tx_recv_cnt(&self) -> usize {
        self.tx_recv_cnt.load(Ordering::Relaxed)
    }
}
