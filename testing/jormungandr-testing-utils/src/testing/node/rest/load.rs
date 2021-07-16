use super::{JormungandrRest, RestError};
use jortestkit::load::{Request, RequestFailure, RequestGenerator};
use rand::RngCore;
use rand_core::OsRng;
use std::time::Instant;

#[derive(Clone)]
pub struct RestRequestGen {
    rest_client: JormungandrRest,
    rand: OsRng,
    addresses: Vec<String>,
    stake_pools: Vec<String>,
}

impl RestRequestGen {
    pub fn new(rest_client: JormungandrRest) -> Self {
        Self {
            rest_client,
            rand: OsRng,
            addresses: Vec::new(),
            stake_pools: Vec::new(),
        }
    }

    pub fn do_setup(&mut self, addresses: Vec<String>) -> Result<(), RestError> {
        self.addresses = addresses;
        //   self.stake_pools = self.rest_client.stake_pools()?;
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
        match self.next_usize() % 10 {
            0 => {
                self.rest_client.p2p_available().map_err(|e| {
                    RequestFailure::General(format!("Rest - p2p_available: {}", e.to_string()))
                })?;
            }
            1 => {
                self.rest_client.fragment_logs().map_err(|e| {
                    RequestFailure::General(format!("Rest - fragment_logs: {}", e.to_string()))
                })?;
            }
            2 => {
                self.rest_client.leaders_log().map_err(|e| {
                    RequestFailure::General(format!("Rest - leaders_log: {}", e.to_string()))
                })?;
            }
            3 => {
                self.rest_client.reward_history(1).map_err(|e| {
                    RequestFailure::General(format!("Rest - reward_history: {}", e.to_string()))
                })?;
            }
            4 => {
                self.rest_client.vote_plan_statuses().map_err(|e| {
                    RequestFailure::General(format!("Rest - vote_plan_statuses: {}", e.to_string()))
                })?;
            }
            5 => {
                self.rest_client.reward_history(1).map_err(|e| {
                    RequestFailure::General(format!("Rest - reward_history: {}", e.to_string()))
                })?;
            }
            6 => {
                self.rest_client.leaders().map_err(|e| {
                    RequestFailure::General(format!("Rest - leaders: {}", e.to_string()))
                })?;
            }
            7 => {
                self.rest_client.stats().map_err(|e| {
                    RequestFailure::General(format!("Rest - stats: {}", e.to_string()))
                })?;
            }
            8 => {
                self.rest_client.network_stats().map_err(|e| {
                    RequestFailure::General(format!("Rest - network_stats: {}", e.to_string()))
                })?;
            }
            9 => {
                self.rest_client.tip().map_err(|e| {
                    RequestFailure::General(format!("Rest - tip: {}", e.to_string()))
                })?;
            }
            _ => unreachable!(),
        }
        Ok(Request {
            ids: vec![None],
            duration: start.elapsed(),
        })
    }

    fn split(self) -> (Self, Option<Self>) {
        (self.clone(), Some(self))
    }
}
