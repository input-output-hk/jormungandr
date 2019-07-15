use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, PoisonError, RwLock};
use std::time::{Instant, SystemTime};

#[derive(Clone, Debug, Default)]
pub struct StatsCounter {
    stats: Arc<StatsCounterImpl>,
}

#[derive(Debug)]
struct StatsCounterImpl {
    tx_recv_cnt: AtomicUsize,
    block_recv_cnt: AtomicUsize,
    start_time: Instant,
    slot_start_time: RwLock<Option<SystemTime>>,
}

impl Default for StatsCounterImpl {
    fn default() -> Self {
        Self {
            tx_recv_cnt: AtomicUsize::default(),
            block_recv_cnt: AtomicUsize::default(),
            start_time: Instant::now(),
            slot_start_time: RwLock::default(),
        }
    }
}

impl StatsCounter {
    pub fn add_tx_recv_cnt(&self, count: usize) {
        self.stats.tx_recv_cnt.fetch_add(count, Ordering::Relaxed);
    }

    pub fn tx_recv_cnt(&self) -> u64 {
        self.stats.tx_recv_cnt.load(Ordering::Relaxed) as u64
    }

    pub fn add_block_recv_cnt(&self, count: usize) {
        self.stats
            .block_recv_cnt
            .fetch_add(count, Ordering::Relaxed);
    }

    pub fn block_recv_cnt(&self) -> u64 {
        self.stats.block_recv_cnt.load(Ordering::Relaxed) as u64
    }

    pub fn uptime_sec(&self) -> u64 {
        self.stats.start_time.elapsed().as_secs()
    }

    pub fn set_slot_start_time(&self, time: SystemTime) {
        self.stats
            .slot_start_time
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(time);
    }

    pub fn slot_start_time(&self) -> Option<SystemTime> {
        *self
            .stats
            .slot_start_time
            .read()
            .unwrap_or_else(PoisonError::into_inner)
    }
}
