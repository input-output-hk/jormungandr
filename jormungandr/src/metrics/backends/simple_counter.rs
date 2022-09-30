use crate::{blockchain::Ref, metrics::MetricsBackend};
use arc_swap::ArcSwapOption;
use chain_impl_mockchain::{
    block::Block,
    fragment::Fragment,
    transaction::Transaction,
    value::{Value, ValueError},
};
use jormungandr_lib::{
    interfaces::NodeStats,
    time::{SecondsSinceUnixEpoch, SystemTime},
};
use std::{
    convert::TryInto,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::Instant,
};

const EXP_MOVING_AVERAGE_COEFF: f64 = 0.5;

pub struct SimpleCounter {
    tx_recv_cnt: AtomicUsize,
    tx_rejected_cnt: AtomicUsize,
    // no atomics for float in the std and bit-fiddling
    // to re-use an AtomicU64 for the porpose
    // seems like unneded complexity for this case
    mempool_usage_ratio: RwLock<f64>,
    mempool_total_size: AtomicUsize,
    votes_cast: AtomicU64,
    block_recv_cnt: AtomicUsize,
    slot_start_time: AtomicU64,
    peers_connected_cnt: AtomicUsize,
    peers_quarantined_cnt: AtomicUsize,
    peers_available_cnt: AtomicUsize,
    tip_block: ArcSwapOption<BlockCounters>,
    start_time: Instant,
}

struct BlockCounters {
    block_tx_count: u64,
    block_input_sum: u64,
    block_fee_sum: u64,
    content_size: u32,
    avg_content_size: f64,
    date: String,
    hash: String,
    chain_length: String,
    time: SystemTime,
}

impl SimpleCounter {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_stats(&self) -> NodeStats {
        let peer_available_cnt = self.peers_available_cnt.load(Ordering::Relaxed);
        let peer_quarantined_cnt = self.peers_quarantined_cnt.load(Ordering::Relaxed);
        let peer_total_cnt = peer_available_cnt + peer_quarantined_cnt;

        let block_data = self.tip_block.load();
        let block_data = block_data.as_deref();

        NodeStats {
            block_recv_cnt: self
                .block_recv_cnt
                .load(Ordering::Relaxed)
                .try_into()
                .unwrap(),
            last_block_content_size: block_data.map(|bd| bd.content_size).unwrap_or_default(),
            last_block_date: block_data.map(|bd| bd.date.clone()),
            last_block_fees: block_data.map(|bd| bd.block_fee_sum).unwrap_or_default(),
            last_block_hash: block_data.map(|bd| bd.hash.clone()),
            last_block_height: block_data.map(|bd| bd.chain_length.clone()),
            last_block_sum: block_data.map(|bd| bd.block_input_sum).unwrap_or_default(),
            last_block_time: block_data.map(|bd| bd.time),
            last_block_tx: block_data.map(|bd| bd.block_tx_count).unwrap_or_default(),
            last_received_block_time: Some(SystemTime::from_secs_since_epoch(
                self.slot_start_time.load(Ordering::Relaxed),
            )),
            block_content_size_avg: block_data.map(|bd| bd.avg_content_size).unwrap_or_default(),
            peer_available_cnt,
            peer_connected_cnt: self.peers_connected_cnt.load(Ordering::Relaxed),
            peer_quarantined_cnt,
            peer_total_cnt,
            tx_recv_cnt: self.tx_recv_cnt.load(Ordering::Relaxed).try_into().unwrap(),
            mempool_usage_ratio: *self.mempool_usage_ratio.read().unwrap(),
            mempool_total_size: self
                .mempool_total_size
                .load(Ordering::Relaxed)
                .try_into()
                .unwrap(),
            tx_rejected_cnt: self
                .tx_rejected_cnt
                .load(Ordering::Relaxed)
                .try_into()
                .unwrap(),
            votes_cast: self.votes_cast.load(Ordering::Relaxed),
            uptime: Some(self.start_time.elapsed().as_secs()),
        }
    }
}

impl Default for SimpleCounter {
    fn default() -> Self {
        Self {
            tx_recv_cnt: Default::default(),
            tx_rejected_cnt: Default::default(),
            mempool_usage_ratio: Default::default(),
            mempool_total_size: Default::default(),
            votes_cast: Default::default(),
            block_recv_cnt: Default::default(),
            slot_start_time: Default::default(),
            peers_connected_cnt: Default::default(),
            peers_quarantined_cnt: Default::default(),
            peers_available_cnt: Default::default(),
            tip_block: Default::default(),
            start_time: Instant::now(),
        }
    }
}

fn calc_running_block_size_average(last_avg: f64, new_value: f64) -> f64 {
    last_avg * (1.0 - EXP_MOVING_AVERAGE_COEFF) + new_value * EXP_MOVING_AVERAGE_COEFF
}

impl MetricsBackend for SimpleCounter {
    fn add_tx_recv_cnt(&self, count: usize) {
        self.tx_recv_cnt.fetch_add(count, Ordering::Relaxed);
    }

    fn add_tx_rejected_cnt(&self, count: usize) {
        self.tx_rejected_cnt.fetch_add(count, Ordering::Relaxed);
    }

    fn set_mempool_usage_ratio(&self, ratio: f64) {
        *self.mempool_usage_ratio.write().unwrap() = ratio;
    }

    fn set_mempool_total_size(&self, size: usize) {
        self.mempool_total_size.store(size, Ordering::Relaxed);
    }

    fn add_block_recv_cnt(&self, count: usize) {
        self.block_recv_cnt.fetch_add(count, Ordering::Relaxed);
    }

    fn add_peer_connected_cnt(&self, count: usize) {
        self.peers_connected_cnt.fetch_add(count, Ordering::Relaxed);
    }

    fn sub_peer_connected_cnt(&self, count: usize) {
        self.peers_connected_cnt.fetch_sub(count, Ordering::Relaxed);
    }

    fn add_peer_quarantined_cnt(&self, count: usize) {
        self.peers_quarantined_cnt
            .fetch_add(count, Ordering::Relaxed);
    }

    fn sub_peer_quarantined_cnt(&self, count: usize) {
        self.peers_quarantined_cnt
            .fetch_sub(count, Ordering::Relaxed);
    }

    fn set_peer_available_cnt(&self, count: usize) {
        self.peers_available_cnt.store(count, Ordering::Relaxed);
    }

    fn set_slot_start_time(&self, time: SecondsSinceUnixEpoch) {
        self.slot_start_time
            .store(time.to_secs(), Ordering::Relaxed);
    }

    fn set_tip_block(&self, block: &Block, block_ref: &Ref) {
        let mut block_tx_count = 0;
        let mut block_input_sum = Value::zero();
        let mut block_fee_sum = Value::zero();
        let mut votes_cast = 0;

        block
            .contents()
            .iter()
            .try_for_each::<_, Result<(), ValueError>>(|fragment| {
                fn totals<T>(t: &Transaction<T>) -> Result<(Value, Value), ValueError> {
                    Ok((t.total_input()?, t.total_output()?))
                }

                let (total_input, total_output) = match &fragment {
                    Fragment::Transaction(tx) => totals(tx),
                    Fragment::OwnerStakeDelegation(tx) => totals(tx),
                    Fragment::StakeDelegation(tx) => totals(tx),
                    Fragment::PoolRegistration(tx) => totals(tx),
                    Fragment::PoolRetirement(tx) => totals(tx),
                    Fragment::PoolUpdate(tx) => totals(tx),
                    Fragment::VotePlan(tx) => totals(tx),
                    Fragment::VoteCast(tx) => {
                        votes_cast += 1;
                        totals(tx)
                    }
                    Fragment::VoteTally(tx) => totals(tx),
                    Fragment::MintToken(tx) => totals(tx),
                    Fragment::UpdateProposal(tx) => totals(tx),
                    Fragment::UpdateVote(tx) => totals(tx),
                    Fragment::EvmMapping(tx) => totals(tx),
                    Fragment::Initial(_) | Fragment::OldUtxoDeclaration(_) | Fragment::Evm(_) => {
                        return Ok(())
                    }
                }?;
                block_tx_count += 1;
                block_input_sum = (block_input_sum + total_input)?;
                let fee = (total_input - total_output).unwrap_or_else(|_| Value::zero());
                block_fee_sum = (block_fee_sum + fee)?;
                Ok(())
            })
            .expect("should be good");

        let content_size = block.header().block_content_size();
        let content_size_ratio =
            content_size as f64 / block_ref.ledger().settings().block_content_max_size as f64;
        let last_avg = if let Some(data) = self.tip_block.load().as_deref() {
            data.avg_content_size
        } else {
            content_size_ratio // jump start moving average from first known value
        };

        let block_data = BlockCounters {
            block_tx_count,
            block_input_sum: block_input_sum.0,
            block_fee_sum: block_fee_sum.0,
            content_size,
            avg_content_size: calc_running_block_size_average(last_avg, content_size_ratio),
            date: block.header().block_date().to_string(),
            hash: block.header().hash().to_string(),
            chain_length: block.header().chain_length().to_string(),
            time: SystemTime::from(block_ref.time()),
        };

        self.votes_cast.fetch_add(votes_cast, Ordering::Relaxed);
        self.tip_block.store(Some(Arc::new(block_data)));
    }
}
