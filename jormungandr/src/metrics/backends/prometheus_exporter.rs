use crate::metrics::MetricsBackend;
use arc_swap::ArcSwapOption;
use chain_impl_mockchain::{
    block::BlockContentHash,
    fragment::Fragment,
    transaction::Transaction,
    value::{Value, ValueError},
};
use prometheus::{
    core::{AtomicU64, GenericGauge},
    Encoder, Gauge, IntCounter, Registry, TextEncoder,
};
use std::{convert::TryInto, sync::Arc, time::SystemTime};

type UIntGauge = GenericGauge<AtomicU64>;

pub struct Prometheus {
    registry: Registry,

    tx_recv_cnt: IntCounter,
    tx_rejected_cnt: IntCounter,
    mempool_usage_ratio: Gauge,
    mempool_size_bytes_total: UIntGauge,
    votes_casted_cnt: IntCounter,
    block_recv_cnt: IntCounter,
    peer_connected_cnt: UIntGauge,
    peer_quarantined_cnt: UIntGauge,
    peer_available_cnt: UIntGauge,
    peer_total_cnt: UIntGauge,
    slot_start_time: UIntGauge,
    block_tx_count: UIntGauge,
    block_input_sum: UIntGauge,
    block_fee_sum: UIntGauge,
    block_content_size: UIntGauge,
    block_epoch: UIntGauge,
    block_slot: UIntGauge,
    block_chain_length: UIntGauge,
    block_time: UIntGauge,
    block_hash: Vec<UIntGauge>,

    block_hash_value: ArcSwapOption<BlockContentHash>,
}

impl Prometheus {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn http_response(&self) -> Result<impl warp::Reply, warp::Rejection> {
        if let Some(block_hash) = self.block_hash_value.load_full() {
            let block_hash_bytes = block_hash.as_bytes();
            for i in 0..4 {
                let mut value_bytes = [0u8; 8];
                value_bytes.copy_from_slice(&block_hash_bytes[(8 * i)..(8 * (i + 1))]);
                let value = u64::from_le_bytes(value_bytes);
                self.block_hash[i].set(value);
            }
            // reset to None because we do not want to trigger these computations for the same block
            // hash value once again
            self.block_hash_value.store(None);
        }
        self.peer_total_cnt
            .set(self.peer_available_cnt.get() + self.peer_quarantined_cnt.get());
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        Ok(warp::http::Response::builder()
            .header("content-type", encoder.format_type())
            .body(buffer))
    }
}

impl Default for Prometheus {
    fn default() -> Self {
        let registry = Registry::new_custom(Some("jormungandr".to_string()), None)
            .expect("failed to create the Prometheus registry");

        let tx_recv_cnt = IntCounter::new("txRecvCnt", "txRecvCnt").unwrap();
        registry.register(Box::new(tx_recv_cnt.clone())).unwrap();
        let mempool_usage_ratio = Gauge::new("mempoolUsageRatio", "mempoolUsageRatio").unwrap();
        registry
            .register(Box::new(mempool_usage_ratio.clone()))
            .unwrap();
        let mempool_size_bytes_total =
            UIntGauge::new("mempoolSizeBytesTotal", "mempoolSizeBytesTotal").unwrap();
        registry
            .register(Box::new(mempool_size_bytes_total.clone()))
            .unwrap();
        let tx_rejected_cnt = IntCounter::new("txRejectedCnt", "txRejectedCnt").unwrap();
        registry
            .register(Box::new(tx_rejected_cnt.clone()))
            .unwrap();
        let votes_casted_cnt = IntCounter::new("votesCasted", "votesCasted").unwrap();
        registry
            .register(Box::new(votes_casted_cnt.clone()))
            .unwrap();
        let block_recv_cnt = IntCounter::new("blockRecvCnt", "blockRecvCnt").unwrap();
        registry.register(Box::new(block_recv_cnt.clone())).unwrap();
        let peer_connected_cnt = UIntGauge::new("peerConnectedCnt", "peerConnectedCnt").unwrap();
        registry
            .register(Box::new(peer_connected_cnt.clone()))
            .unwrap();
        let peer_quarantined_cnt =
            UIntGauge::new("peerQuarantinedCnt", "peerQuarantinedCnt").unwrap();
        registry
            .register(Box::new(peer_quarantined_cnt.clone()))
            .unwrap();
        let peer_available_cnt = UIntGauge::new("peerAvailableCnt", "peerAvailableCnt").unwrap();
        registry
            .register(Box::new(peer_available_cnt.clone()))
            .unwrap();
        let peer_total_cnt = UIntGauge::new("peerTotalCnt", "peerTotalCnt").unwrap();
        registry.register(Box::new(peer_total_cnt.clone())).unwrap();
        let slot_start_time =
            UIntGauge::new("lastReceivedBlockTime", "lastReceivedBlockTime").unwrap();
        registry
            .register(Box::new(slot_start_time.clone()))
            .unwrap();
        let block_tx_count = UIntGauge::new("lastBlockTx", "lastBlockTx").unwrap();
        registry.register(Box::new(block_tx_count.clone())).unwrap();
        let block_input_sum = UIntGauge::new("lastBlockInputTime", "lastBlockInputTime").unwrap();
        registry
            .register(Box::new(block_input_sum.clone()))
            .unwrap();
        let block_fee_sum = UIntGauge::new("lastBlockSum", "lastBlockSum").unwrap();
        registry.register(Box::new(block_fee_sum.clone())).unwrap();
        let block_content_size =
            UIntGauge::new("lastBlockContentSize", "lastBlockContentSize").unwrap();
        registry
            .register(Box::new(block_content_size.clone()))
            .unwrap();
        let block_epoch = UIntGauge::new("lastBlockEpoch", "lastBlockEpoch").unwrap();
        registry.register(Box::new(block_epoch.clone())).unwrap();
        let block_slot = UIntGauge::new("lastBlockSlot", "lastBlockSlot").unwrap();
        registry.register(Box::new(block_slot.clone())).unwrap();
        let block_chain_length = UIntGauge::new("lastBlockHeight", "lastBlockHeight").unwrap();
        registry
            .register(Box::new(block_chain_length.clone()))
            .unwrap();
        let block_time = UIntGauge::new("lastBlockDate", "lastBlockDate").unwrap();
        registry.register(Box::new(block_time.clone())).unwrap();

        let block_hash = {
            let mut pcs = Vec::new();
            for i in 1..=4 {
                let name = format!("lastBlockHashPiece{}", i);
                let gauge = UIntGauge::new(&name, &name).unwrap();
                registry.register(Box::new(gauge.clone())).unwrap();
                pcs.push(gauge);
            }
            pcs
        };

        Self {
            registry,
            tx_recv_cnt,
            tx_rejected_cnt,
            mempool_usage_ratio,
            mempool_size_bytes_total,
            votes_casted_cnt,
            block_recv_cnt,
            peer_connected_cnt,
            peer_quarantined_cnt,
            peer_available_cnt,
            peer_total_cnt,
            slot_start_time,
            block_tx_count,
            block_input_sum,
            block_fee_sum,
            block_content_size,
            block_epoch,
            block_slot,
            block_chain_length,
            block_time,
            block_hash,
            block_hash_value: Default::default(),
        }
    }
}

impl MetricsBackend for Prometheus {
    fn add_tx_recv_cnt(&self, count: usize) {
        let count = count.try_into().unwrap();
        self.tx_recv_cnt.inc_by(count);
    }

    fn add_tx_rejected_cnt(&self, count: usize) {
        let count = count.try_into().unwrap();
        self.tx_rejected_cnt.inc_by(count);
    }

    fn set_mempool_usage_ratio(&self, ratio: f64) {
        self.mempool_usage_ratio.set(ratio);
    }

    fn set_mempool_total_size(&self, size: usize) {
        let size = size.try_into().unwrap();
        self.mempool_size_bytes_total.set(size);
    }

    fn add_block_recv_cnt(&self, count: usize) {
        let count = count.try_into().unwrap();
        self.block_recv_cnt.inc_by(count);
    }

    fn add_peer_connected_cnt(&self, count: usize) {
        let count = count.try_into().unwrap();
        self.peer_connected_cnt.add(count);
    }

    fn sub_peer_connected_cnt(&self, count: usize) {
        let count = count.try_into().unwrap();
        self.peer_connected_cnt.sub(count);
    }

    fn add_peer_quarantined_cnt(&self, count: usize) {
        let count = count.try_into().unwrap();
        self.peer_quarantined_cnt.add(count);
    }

    fn sub_peer_quarantined_cnt(&self, count: usize) {
        let count = count.try_into().unwrap();
        self.peer_quarantined_cnt.sub(count);
    }

    fn set_peer_available_cnt(&self, count: usize) {
        let count = count.try_into().unwrap();
        self.peer_available_cnt.set(count);
    }

    fn set_slot_start_time(&self, time: jormungandr_lib::time::SecondsSinceUnixEpoch) {
        self.slot_start_time.set(time.to_secs());
    }

    fn set_tip_block(
        &self,
        block: &chain_impl_mockchain::block::Block,
        block_ref: &crate::blockchain::Ref,
    ) {
        let mut block_tx_count = 0;
        let mut block_input_sum = Value::zero();
        let mut block_fee_sum = Value::zero();
        let mut votes_casted = 0;

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
                        votes_casted += 1;
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

        self.votes_casted_cnt.inc_by(votes_casted);
        self.block_tx_count.set(block_tx_count);
        self.block_input_sum.set(block_input_sum.0);
        self.block_fee_sum.set(block_fee_sum.0);
        self.block_content_size
            .set(block.header().block_content_size().into());
        self.block_epoch
            .set(block.header().block_date().epoch.into());
        self.block_slot
            .set(block.header().block_date().slot_id.into());
        let chain_length: u32 = block.header().chain_length().try_into().unwrap();
        self.block_chain_length.set(chain_length as u64);
        self.block_time.set(
            block_ref
                .time()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );

        let block_hash = block.header().hash();
        self.block_hash_value.store(Some(Arc::new(block_hash)));
    }
}
