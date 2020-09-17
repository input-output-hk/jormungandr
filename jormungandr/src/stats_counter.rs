use arc_swap::ArcSwapOption;
use chain_impl_mockchain::block::Block;
use jormungandr_lib::time::SecondsSinceUnixEpoch;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
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
    tip_block: ArcSwapOption<Block>,
    peers_connected_cnt: AtomicUsize,
}

impl Default for StatsCounterImpl {
    fn default() -> Self {
        Self {
            tx_recv_cnt: AtomicUsize::default(),
            block_recv_cnt: AtomicUsize::default(),
            start_time: Instant::now(),
            slot_start_time: AtomicU64::new(SLOT_START_TIME_UNDEFINED),
            tip_block: ArcSwapOption::from(None),
            peers_connected_cnt: AtomicUsize::default(),
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

    pub fn add_peer_connected_cnt(&self, count: usize) -> usize {
        self.stats
            .peers_connected_cnt
            .fetch_add(count, Ordering::SeqCst)
    }

    pub fn sub_peer_connected_cnt(&self, count: usize) -> usize {
        self.stats
            .peers_connected_cnt
            .fetch_sub(count, Ordering::SeqCst)
    }

    pub fn peer_connected_cnt(&self) -> usize {
        self.stats.peers_connected_cnt.load(Ordering::Relaxed)
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

    pub fn set_tip_block(&self, block: Arc<Block>) {
        self.stats.tip_block.swap(Some(block));
    }

    pub fn get_tip_block(&self) -> Option<Arc<Block>> {
        self.stats.tip_block.load_full()
    }
}
