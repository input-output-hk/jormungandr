use chain_impl_mockchain::block::Block;
use jormungandr_lib::time::SecondsSinceUnixEpoch;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio02::sync::RwLock;
use std::time::Instant;

const SLOT_START_TIME_UNDEFINED: u64 = u64::max_value();

#[derive(Clone, Debug, Default)]
pub struct StatsCounter {
    stats: Arc<StatsCounterImpl>,
}

#[derive(Debug)]
struct StatsCounterImpl {
    tx_recv_cnt: AtomicUsize,
    block_recv_cnt: AtomicUsize,
    start_time: Instant,
    slot_start_time: AtomicU64,
    tip_block: RwLock<Arc<Option<Block>>>,
}

impl Default for StatsCounterImpl {
    fn default() -> Self {
        Self {
            tx_recv_cnt: AtomicUsize::default(),
            block_recv_cnt: AtomicUsize::default(),
            start_time: Instant::now(),
            slot_start_time: AtomicU64::new(SLOT_START_TIME_UNDEFINED),
            tip_block: RwLock::new(Arc::new(None)),
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
        self.set_slot_start_time(SecondsSinceUnixEpoch::now());
    }

    pub fn block_recv_cnt(&self) -> u64 {
        self.stats.block_recv_cnt.load(Ordering::Relaxed) as u64
    }

    pub fn uptime_sec(&self) -> u64 {
        self.stats.start_time.elapsed().as_secs()
    }

    pub fn set_slot_start_time(&self, time: SecondsSinceUnixEpoch) {
        self.stats
            .slot_start_time
            .store(time.to_secs(), Ordering::Relaxed)
    }

    /// this value does returns the time of the last received block
    ///
    /// This is not the time of the block within the blockchain
    pub fn slot_start_time(&self) -> Option<SecondsSinceUnixEpoch> {
        match self.stats.slot_start_time.load(Ordering::Relaxed) {
            SLOT_START_TIME_UNDEFINED => None,
            slot_start_time => Some(slot_start_time),
        }
        .map(SecondsSinceUnixEpoch::from_secs)
    }

    pub async fn set_tip_block(&self, block: Option<Block>) {
        *self.stats.tip_block.write().await = Arc::new(block);
    }

    pub async fn get_tip_block(&self) -> Arc<Option<Block>> {
        self.stats.tip_block.read().await.clone()
    }
}
