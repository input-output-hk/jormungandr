use jormungandr_automation::jormungandr::{JormungandrRest, RestError};
use jortestkit::load::{Request, RequestFailure, RequestGenerator};
use rand::RngCore;
use rand_core::OsRng;
use std::time::Instant;

const DEFAULT_MAX_SPLITS: usize = 7; // equals to 128 splits, will likely not reach that value but it's there just to prevent a stack overflow

#[derive(Clone)]
pub struct RestRequestGen {
    rest_client: JormungandrRest,
    rand: OsRng,
    addresses: Vec<String>,
    stake_pools: Vec<String>,
    max_splits: usize,
}

impl RestRequestGen {
    pub fn new(rest_client: JormungandrRest) -> Self {
        Self {
            rest_client,
            rand: OsRng,
            addresses: Vec::new(),
            stake_pools: Vec::new(),
            max_splits: DEFAULT_MAX_SPLITS,
        }
    }

    pub fn do_setup(&mut self, addresses: Vec<String>) -> Result<(), RestError> {
        self.addresses = addresses;
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

impl RequestGenerator for RestRequestGen {
    fn next(&mut self) -> Result<Request, RequestFailure> {
        let start = Instant::now();
        match self.next_usize() % 9 {
            0 => {
                self.rest_client
                    .p2p_available()
                    .map_err(|e| RequestFailure::General(format!("Rest - p2p_available: {}", e)))?;
            }
            1 => {
                self.rest_client
                    .fragment_logs()
                    .map_err(|e| RequestFailure::General(format!("Rest - fragment_logs: {}", e)))?;
            }
            2 => {
                self.rest_client
                    .leaders_log()
                    .map_err(|e| RequestFailure::General(format!("Rest - leaders_log: {}", e)))?;
            }
            3 => {
                self.rest_client.reward_history(1).map_err(|e| {
                    RequestFailure::General(format!("Rest - reward_history: {}", e))
                })?;
            }
            4 => {
                self.rest_client.vote_plan_statuses().map_err(|e| {
                    RequestFailure::General(format!("Rest - vote_plan_statuses: {}", e))
                })?;
            }
            5 => {
                self.rest_client.reward_history(1).map_err(|e| {
                    RequestFailure::General(format!("Rest - reward_history: {}", e))
                })?;
            }
            6 => {
                self.rest_client
                    .stats()
                    .map_err(|e| RequestFailure::General(format!("Rest - stats: {}", e)))?;
            }
            7 => {
                self.rest_client
                    .network_stats()
                    .map_err(|e| RequestFailure::General(format!("Rest - network_stats: {}", e)))?;
            }
            8 => {
                self.rest_client
                    .tip()
                    .map_err(|e| RequestFailure::General(format!("Rest - tip: {}", e)))?;
            }
            _ => unreachable!(),
        }
        Ok(Request {
            ids: vec![None],
            duration: start.elapsed(),
        })
    }

    fn split(mut self) -> (Self, Option<Self>) {
        // Since rest queries do not modify the node state we can split as many times as we want
        // but that may trigger a bug in rayon so we artificially limit it
        if self.max_splits == 0 {
            return (self, None);
        }
        self.max_splits -= 1;
        (self.clone(), Some(self))
    }
}
