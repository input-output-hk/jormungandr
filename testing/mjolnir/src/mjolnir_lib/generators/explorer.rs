use jormungandr_automation::jormungandr::{Explorer, ExplorerError};
use jortestkit::load::{Request, RequestFailure, RequestGenerator};
use rand::RngCore;
use rand_core::OsRng;
use std::time::Instant;

const DEFAULT_MAX_SPLITS: usize = 7; // equals to 128 splits, will likely not reach that value but it's there just to prevent a stack overflow

#[derive(Clone)]
pub struct ExplorerRequestGen {
    explorer: Explorer,
    rand: OsRng,
    addresses: Vec<String>,
    stake_pools: Vec<String>,
    max_splits: usize,
}

impl ExplorerRequestGen {
    pub fn new(explorer: Explorer) -> Self {
        Self {
            explorer,
            rand: OsRng,
            addresses: Vec::new(),
            stake_pools: Vec::new(),
            max_splits: DEFAULT_MAX_SPLITS,
        }
    }

    pub fn do_setup(&mut self, addresses: Vec<String>) -> Result<(), ExplorerError> {
        self.addresses = addresses;
        let stake_pools = self.explorer.stake_pools(1000)?;
        let explorer_stake_pools = stake_pools.data.unwrap().tip.all_stake_pools.edges;
        self.stake_pools = explorer_stake_pools
            .iter()
            .map(|edge| edge.node.id.clone())
            .collect::<Vec<String>>();
        Ok(())
    }

    pub fn next_usize(&mut self) -> usize {
        self.rand.next_u32() as usize
    }

    pub fn next_usize_in_range(&mut self, lower: usize, upper: usize) -> usize {
        self.next_usize() % (upper - lower) + lower
    }

    pub fn next_address(&mut self) -> Option<&String> {
        if self.addresses.is_empty() {
            return None;
        }

        let next_address = self.next_usize() % self.addresses.len();
        self.addresses.get(next_address)
    }

    pub fn next_pool_id(&mut self) -> Option<&String> {
        if self.stake_pools.is_empty() {
            return None;
        }

        let next_stake_pool_id = self.next_usize() % self.stake_pools.len();
        self.stake_pools.get(next_stake_pool_id)
    }
}

impl RequestGenerator for ExplorerRequestGen {
    fn next(&mut self) -> Result<Request, RequestFailure> {
        let start = Instant::now();
        let result = match self.next_usize() % 7 {
            0 => {
                let limit = self.next_usize_in_range(1, 10) as i64;
                self.explorer
                    .stake_pools(limit)
                    .map(|_| ())
                    .map_err(|e| RequestFailure::General(format!("Explorer - StakePools: {:?}", e)))
            }
            1 => {
                let limit = self.next_usize_in_range(1, 10) as i64;
                self.explorer
                    .blocks(limit)
                    .map(|_| ())
                    .map_err(|e| RequestFailure::General(format!("Explorer- Blocks: {:?}", e)))
            }
            2 => self
                .explorer
                .last_block()
                .map(|_| ())
                .map_err(|e| RequestFailure::General(format!("Explorer - LastBlock: {:?}", e))),
            3 => {
                let limit = self.next_usize_in_range(1, 30) as u32;
                self.explorer
                    .blocks_at_chain_length(limit)
                    .map(|_| ())
                    .map_err(|e| {
                        RequestFailure::General(format!("Explorer - BlockAtChainLength: {:?}", e))
                    })
            }
            4 => {
                let epoch_nr = self.next_usize_in_range(1, 30) as u32;
                let limit = self.next_usize_in_range(1, 30) as i64;
                self.explorer
                    .epoch(epoch_nr, limit)
                    .map(|_| ())
                    .map_err(|e| RequestFailure::General(format!("Explorer - Epoch: {:?}", e)))
            }
            5 => {
                let explorer = self.explorer.clone();
                let limit = self.next_usize_in_range(1, 1000) as i64;
                if let Some(pool_id) = self.next_pool_id() {
                    explorer
                        .stake_pool(pool_id.to_string(), limit)
                        .map(|_| ())
                        .map_err(|e| {
                            RequestFailure::General(format!("Explorer - StakePool: {:?}", e))
                        })
                } else {
                    explorer
                        .settings()
                        .map(|_| ())
                        .map_err(|e| RequestFailure::General(format!("Status: {:?}", e)))
                }
            }
            6 => self
                .explorer
                .settings()
                .map(|_| ())
                .map_err(|e| RequestFailure::General(format!("Status: {:?}", e))),
            7 => {
                let limit = self.next_usize_in_range(1, 1000) as i64;
                self.explorer
                    .vote_plans(limit)
                    .map(|_| ())
                    .map_err(|e| RequestFailure::General(format!("Explorer - VotePlans: {:?}", e)))
            }
            8 => {
                let explorer = self.explorer.clone();
                if let Some(pool_id) = self.next_address() {
                    explorer.address(pool_id).map(|_| ()).map_err(|e| {
                        RequestFailure::General(format!("Explorer - Address: {:?}", e))
                    })
                } else {
                    explorer
                        .settings()
                        .map(|_| ())
                        .map_err(|e| RequestFailure::General(format!("Status: {:?}", e)))
                }
            }
            _ => unreachable!(),
        };
        result.map(|()| Request {
            ids: vec![None],
            duration: start.elapsed(),
        })
    }

    fn split(mut self) -> (Self, Option<Self>) {
        // Since explorer queries do not modify the node state we can split as many times as we want
        // but that may trigger a bug in rayon so we artificially limit it
        if self.max_splits == 0 {
            return (self, None);
        }
        self.max_splits -= 1;
        (self.clone(), Some(self))
    }
}
