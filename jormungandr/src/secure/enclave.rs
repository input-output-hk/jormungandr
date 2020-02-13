use crate::blockcfg::{
    BlockDate, HeaderBft, HeaderBftBuilder, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
    HeaderSetConsensusSignature, SlotId,
};
use chain_impl_mockchain::leadership::{Leader, LeaderOutput, Leadership};
use jormungandr_lib::interfaces::EnclaveLeaderId as LeaderId;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tokio02::sync::RwLock;

struct EnclaveLeadersWithCache {
    leaders: BTreeMap<LeaderId, Leader>,
    added_leaders_cache: HashMap<String, LeaderId>,
}

#[derive(Clone)]
pub struct Enclave {
    leaders_data: Arc<RwLock<EnclaveLeadersWithCache>>,
    //    leaders: Arc<RwLock<BTreeMap<LeaderId, Leader>>>,
    //    added_leaders_cache: Arc<RwLock<HashMap<String, LeaderId>>>,
}

pub struct LeaderEvent {
    pub id: LeaderId,
    pub date: BlockDate,
    pub output: LeaderOutput,
}

fn get_maximum_id<A>(leaders: &BTreeMap<LeaderId, A>) -> LeaderId {
    leaders.keys().last().copied().unwrap_or(LeaderId::new())
}

impl Enclave {
    pub fn new() -> Self {
        Enclave {
            leaders_data: Arc::new(RwLock::new(EnclaveLeadersWithCache {
                leaders: BTreeMap::new(),
                added_leaders_cache: HashMap::new(),
            })),
        }
    }

    pub async fn from_vec(leaders: Vec<Leader>) -> Self {
        let e = Self::new();
        for leader in leaders {
            e.add_leader(leader).await;
        }
        e
    }
    async fn get_maximum_id(&self) -> LeaderId {
        let leaders = &self.leaders_data.read().await.leaders;
        get_maximum_id(leaders).next()
    }

    pub async fn get_leader_id_if_present(&self, leader: &Leader) -> Option<LeaderId> {
        let cache = &self.leaders_data.read().await.added_leaders_cache;
        // match protocol leaders prioritizing genesis ones
        match leader {
            Leader {
                bft_leader: None,
                genesis_leader: None,
            } => None,
            Leader {
                bft_leader: None,
                genesis_leader: Some(l),
            } => cache.get(&l.node_id.to_string()).cloned(),
            Leader {
                bft_leader: Some(l),
                genesis_leader: None,
            } => cache.get(&l.sig_key.to_public().to_string()).cloned(),
            Leader {
                bft_leader: Some(_),
                genesis_leader: Some(l),
            } => cache.get(&l.node_id.to_string()).cloned(),
        }
    }

    pub async fn add_leader_to_cache(&self, leader: &Leader, id: LeaderId) {
        let mut cache = &mut self.leaders_data.write().await.added_leaders_cache;
        // match protocol leaders prioritizing genesis ones
        match leader {
            Leader {
                bft_leader: None,
                genesis_leader: None,
            } => (),
            Leader {
                bft_leader: None,
                genesis_leader: Some(l),
            } => {
                cache.insert(l.node_id.to_string(), id);
            }
            Leader {
                bft_leader: Some(l),
                genesis_leader: None,
            } => {
                cache.insert(l.sig_key.to_public().to_string(), id);
            }
            Leader {
                bft_leader: Some(_),
                genesis_leader: Some(l),
            } => {
                cache.insert(l.node_id.to_string(), id);
            }
        }
    }

    pub async fn get_leaderids(&self) -> Vec<LeaderId> {
        let leaders = &self.leaders_data.read().await.leaders;
        leaders.keys().cloned().collect()
    }

    async fn _add_leader(&self, leader: Leader, id: LeaderId) {
        let mut leaders = &mut self.leaders_data.write().await.leaders;
        match leaders.insert(id, leader) {
            None => (),
            // This panic case should never happens in practice, as this structure is
            // not supposed to be shared between thread.
            Some(_) => panic!("enclave leader failed : duplicated value race"),
        };
    }

    pub async fn add_leader(&self, leader: Leader) -> LeaderId {
        match self.get_leader_id_if_present(&leader).await {
            Some(id) => return id,
            None => {}
        }

        let next_leader_id = self.get_maximum_id().await;

        // Add the new leader to the cache
        self.add_leader_to_cache(&leader, next_leader_id).await;
        // Add the new leader
        self._add_leader(leader, next_leader_id).await;
        next_leader_id
    }

    pub async fn remove_leader(&self, leader_id: LeaderId) -> bool {
        let mut leaders = &mut self.leaders_data.write().await.leaders;
        leaders.remove(&leader_id).is_some()
    }

    // temporary method
    pub async fn leadership_evaluate1(
        &self,
        leadership: &Leadership,
        leader_id: &LeaderId,
        slot: SlotId,
    ) -> Option<LeaderEvent> {
        let leaders = &self.leaders_data.read().await.leaders;
        if leaders.len() == 0 {
            return None;
        }

        leaders.get(leader_id).and_then(|leader| {
            let date = leadership.date_at_slot(slot);
            match leadership.is_leader_for_date(&leader, date) {
                Ok(LeaderOutput::None) => None,
                Ok(leader_output) => Some(LeaderEvent {
                    id: *leader_id,
                    date: date,
                    output: leader_output,
                }),
                Err(_) => {
                    // For now silently ignore error
                    None
                }
            }
        })
    }

    pub async fn leadership_evaluate(
        &self,
        leadership: &Leadership,
        slot_start: u32,
        nb_slots: u32,
    ) -> Vec<LeaderEvent> {
        let leaders = &self.leaders_data.read().await.leaders;
        if leaders.len() == 0 {
            return vec![];
        }

        let mut output = Vec::new();
        for slot_idx in slot_start..slot_start + nb_slots {
            let date = leadership.date_at_slot(slot_idx);
            for (id, leader) in leaders.iter() {
                match leadership.is_leader_for_date(&leader, date) {
                    Ok(LeaderOutput::None) => (),
                    Ok(leader_output) => output.push(LeaderEvent {
                        id: *id,
                        date: date,
                        output: leader_output,
                    }),
                    Err(_) => {
                        // For now silently ignore error
                    }
                }
            }
        }
        output
    }

    pub async fn create_header_genesis_praos(
        &self,
        header_builder: HeaderGenesisPraosBuilder<HeaderSetConsensusSignature>,
        id: LeaderId,
    ) -> Option<HeaderGenesisPraos> {
        let leaders = &self.leaders_data.read().await.leaders;
        let leader = leaders.get(&id)?;
        if let Some(genesis_leader) = &leader.genesis_leader {
            let data = header_builder.get_authenticated_data();
            let signature = genesis_leader.sig_key.sign_slice(data);
            Some(header_builder.set_signature(signature.into()))
        } else {
            None
        }
    }

    pub async fn create_header_bft(
        &self,
        header_builder: HeaderBftBuilder<HeaderSetConsensusSignature>,
        id: LeaderId,
    ) -> Option<HeaderBft> {
        let leaders = &self.leaders_data.read().await.leaders;
        let leader = leaders.get(&id)?;
        if let Some(ref leader) = &leader.bft_leader {
            let data = header_builder.get_authenticated_data();
            let signature = leader.sig_key.sign_slice(data);
            Some(header_builder.set_signature(signature.into()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chain_crypto::{Blake2b256, Ed25519, SecretKey};
    use chain_impl_mockchain::certificate::PoolId;
    use chain_impl_mockchain::fragment::Fragment::PoolRegistration;
    use chain_impl_mockchain::leadership::{BftLeader, GenesisLeader};
    use rand_core;
    use tokio02 as tokio;

    #[tokio::test]
    async fn enclave_add_different_bft_leaders() {
        let mut enclave = Enclave::new();
        let rng = rand_core::OsRng;
        let leader1 = Leader {
            bft_leader: Some(BftLeader {
                sig_key: SecretKey::generate(rng),
            }),
            genesis_leader: None,
        };
        let leader2 = Leader {
            bft_leader: Some(BftLeader {
                sig_key: SecretKey::generate(rng),
            }),
            genesis_leader: None,
        };
        let init_leader_id = LeaderId::new();
        let fst_id = init_leader_id.next();
        let snd_id = fst_id.next();
        assert_eq!(enclave.add_leader(leader1).await, fst_id);
        assert_eq!(enclave.add_leader(leader2).await, snd_id);
        let leaders_data = &enclave.leaders_data.read().await;
        assert_eq!(leaders_data.leaders.len(), 2);
        assert_eq!(leaders_data.added_leaders_cache.len(), 2);
    }

    #[tokio::test]
    async fn enclave_add_duplicated_bft_leaders() {
        let mut enclave = Enclave::new();
        let secret_key = SecretKey::generate(rand_core::OsRng);
        let leader1 = Leader {
            bft_leader: Some(BftLeader {
                sig_key: secret_key.clone(),
            }),
            genesis_leader: None,
        };
        let leader2 = Leader {
            bft_leader: Some(BftLeader {
                sig_key: secret_key.clone(),
            }),
            genesis_leader: None,
        };
        // Both leaders are different instances of the same data, adding both of them should return the same id
        assert_eq!(
            enclave.add_leader(leader1).await,
            enclave.add_leader(leader2).await
        );
        // Just one it is really added
        let leaders_data = &enclave.leaders_data.read().await;
        assert_eq!(leaders_data.leaders.len(), 1);
        assert_eq!(leaders_data.added_leaders_cache.len(), 1);
    }

    //    #[tokio::test]
    //    async fn enclave_add_different_genesis_leaders() {
    //        let mut enclave = Enclave::new();
    //        let rng = rand_core::OsRng;
    //        let leader1 = Leader {
    //            bft_leader: None,
    //            genesis_leader: Some( GenesisLeader {
    //                sig_key: SecretKey::generate(rng),
    //                vrf_key: SecretKey::generate(rng),
    //                node_id: PoolId::,
    //            }),
    //        };
    //        let leader2 = Leader {
    //            bft_leader: None,
    //            genesis_leader: None,
    //        };
    //        let init_leader_id = LeaderId::new();
    //        let fst_id = init_leader_id.next();
    //        let snd_id = fst_id.next();
    //        assert_eq!(enclave.add_leader(leader1).await, fst_id);
    //        assert_eq!(enclave.add_leader(leader2).await, snd_id);
    //        assert_eq!(enclave.leaders.read().await.len(), 2);
    //        assert_eq!(enclave.added_leaders_cache.read().await.len(), 2);
    //    }
    //
    //    #[tokio::test]
    //    async fn enclave_add_duplicated_genesis_leaders() {
    //        let mut enclave = Enclave::new();
    //        let secret_key = SecretKey::generate(rand_core::OsRng);
    //        let leader1 = Leader {
    //            bft_leader: Some(BftLeader {
    //                sig_key: secret_key.clone(),
    //            }),
    //            genesis_leader: None,
    //        };
    //        let leader2 = Leader {
    //            bft_leader: Some(BftLeader {
    //                sig_key: secret_key.clone(),
    //            }),
    //            genesis_leader: None,
    //        };
    //        // Both leaders are different instances of the same data, adding both of them should return the same id
    //        assert_eq!(
    //            enclave.add_leader(leader1).await,
    //            enclave.add_leader(leader2).await
    //        );
    //        // Just one it is really added
    //        assert_eq!(enclave.leaders.read().await.len(), 1);
    //        assert_eq!(enclave.added_leaders_cache.read().await.len(), 1);
    //    }
}
