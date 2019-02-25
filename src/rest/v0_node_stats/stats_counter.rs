use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Debug, Default)]
pub struct StatsCounter {
    stats: Arc<StatsCounterImpl>,
}

#[derive(Debug)]
struct StatsCounterImpl {
    tx_recv_cnt: AtomicUsize,
    block_recv_cnt: AtomicUsize,
    start_time: Instant,
}

impl Default for StatsCounterImpl {
    fn default() -> Self {
        Self {
            tx_recv_cnt: AtomicUsize::default(),
            block_recv_cnt: AtomicUsize::default(),
            start_time: Instant::now(),
        }
    }
}

impl StatsCounter {
    pub fn add_tx_recv_cnt(&self, count: usize) {
        self.stats.tx_recv_cnt.fetch_add(count, Ordering::Relaxed);
    }

    pub fn get_tx_recv_cnt(&self) -> u64 {
        self.stats.tx_recv_cnt.load(Ordering::Relaxed) as u64
    }

    pub fn add_block_recv_cnt(&self, count: usize) {
        self.stats
            .block_recv_cnt
            .fetch_add(count, Ordering::Relaxed);
    }

    pub fn get_block_recv_cnt(&self) -> u64 {
        self.stats.block_recv_cnt.load(Ordering::Relaxed) as u64
    }

    pub fn get_uptime_sec(&self) -> u64 {
        self.stats.start_time.elapsed().as_secs()
    }
}
