use crate::blockcfg::{
    BlockDate, HeaderBft, HeaderBftBuilder, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
    HeaderSetConsensusSignature, SlotId,
};
use chain_impl_mockchain::leadership::{Leader, LeaderOutput, Leadership};
use jormungandr_lib::interfaces::EnclaveLeaderId as LeaderId;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
struct EnclaveLeadersWithCache {
    leaders: BTreeMap<LeaderId, Leader>,
    added_leaders_cache: HashMap<String, LeaderId>,
}

#[derive(Clone)]
pub struct Enclave {
    leaders_data: Arc<RwLock<EnclaveLeadersWithCache>>,
}

pub struct LeaderEvent {
    pub id: LeaderId,
    pub date: BlockDate,
    pub output: LeaderOutput,
}

fn get_maximum_id<A>(leaders: &BTreeMap<LeaderId, A>) -> LeaderId {
    leaders.keys().last().copied().unwrap_or(LeaderId::new())
}

fn leader_identifier(leader: &Leader) -> String {
    match leader {
        Leader {
            bft_leader: None,
            genesis_leader: None,
        } => "".to_owned(),
        Leader {
            bft_leader: None,
            genesis_leader: Some(l),
        } => l.node_id.to_string(),
        Leader {
            bft_leader: Some(l),
            genesis_leader: None,
        } => l.sig_key.to_public().to_string(),
        Leader {
            bft_leader: Some(_),
            genesis_leader: Some(l),
        } => l.node_id.to_string(),
    }
}

impl EnclaveLeadersWithCache {
    fn add(&mut self, leader: Leader) -> LeaderId {
        let identifier = leader_identifier(&leader);
        if let Some(leader_id) = self.added_leaders_cache.get(&identifier) {
            *leader_id
        } else {
            let leader_id = get_maximum_id(&self.leaders).next();

            self.added_leaders_cache.insert(identifier, leader_id);
            self.leaders.insert(leader_id, leader);

            leader_id
        }
    }

    fn remove(&mut self, leader_id: LeaderId) -> bool {
        if let Some(leader) = self.leaders.remove(&leader_id) {
            let identifier = leader_identifier(&leader);

            self.added_leaders_cache.remove(&identifier);

            true
        } else {
            false
        }
    }

    fn get_leader_ids(&self) -> Vec<LeaderId> {
        self.added_leaders_cache.values().copied().collect()
    }
}

impl Enclave {
    pub fn new() -> Self {
        Enclave {
            leaders_data: Arc::new(RwLock::new(EnclaveLeadersWithCache::default())),
        }
    }

    pub async fn from_vec(leaders: Vec<Leader>) -> Self {
        let e = Self::new();
        for leader in leaders {
            e.add_leader(leader).await;
        }
        e
    }

    pub async fn get_leader_ids(&self) -> Vec<LeaderId> {
        self.leaders_data.read().await.get_leader_ids()
    }

    pub async fn add_leader(&self, leader: Leader) -> LeaderId {
        self.leaders_data.write().await.add(leader)
    }

    pub async fn remove_leader(&self, leader_id: LeaderId) -> bool {
        self.leaders_data.write().await.remove(leader_id)
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
    use chain_crypto::SecretKey;
    use chain_impl_mockchain::leadership::{BftLeader, GenesisLeader};

    #[tokio::test]
    async fn enclave_add_different_bft_leaders() {
        let enclave = Enclave::new();
        let mut rng = rand_core::OsRng;

        let leader1 = Leader {
            bft_leader: Some(BftLeader {
                sig_key: SecretKey::generate(&mut rng),
            }),
            genesis_leader: None,
        };

        let leader2 = Leader {
            bft_leader: Some(BftLeader {
                sig_key: SecretKey::generate(&mut rng),
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
        let enclave = Enclave::new();
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

    fn mk_pool_id(rng: &mut dyn rand_core::RngCore) -> chain_impl_mockchain::certificate::PoolId {
        let mut bytes = [0; 32];

        rng.fill_bytes(&mut bytes);

        bytes.into()
    }

    #[tokio::test]
    async fn enclave_add_different_genesis_leaders() {
        let enclave = Enclave::new();
        let mut rng = rand_core::OsRng;

        let leader1 = Leader {
            bft_leader: None,
            genesis_leader: Some(GenesisLeader {
                sig_key: SecretKey::generate(&mut rng),
                vrf_key: SecretKey::generate(&mut rng),
                node_id: mk_pool_id(&mut rng),
            }),
        };

        let leader2 = Leader {
            bft_leader: None,
            genesis_leader: Some(GenesisLeader {
                sig_key: SecretKey::generate(&mut rng),
                vrf_key: SecretKey::generate(&mut rng),
                node_id: mk_pool_id(&mut rng),
            }),
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
    async fn enclave_add_duplicated_genesis_leaders() {
        let enclave = Enclave::new();

        let mut rng = rand_core::OsRng;
        let sig_key_1 = SecretKey::generate(&mut rng);
        let sig_key_2 = SecretKey::generate(&mut rng);
        let id = mk_pool_id(&mut rng);

        let leader1 = Leader {
            bft_leader: None,
            genesis_leader: Some(GenesisLeader {
                sig_key: sig_key_1.clone(),
                vrf_key: sig_key_2.clone(),
                node_id: id.clone(),
            }),
        };

        let leader2 = Leader {
            bft_leader: None,
            genesis_leader: Some(GenesisLeader {
                sig_key: sig_key_1,
                vrf_key: sig_key_2,
                node_id: id,
            }),
        };

        // Both leaders are different instances of the same data, adding both of them should return the same id
        assert_eq!(
            enclave.add_leader(leader1).await,
            enclave.add_leader(leader2).await
        );

        let leaders_data = &enclave.leaders_data.read().await;

        // Just one it is really added
        assert_eq!(leaders_data.leaders.len(), 1);
        assert_eq!(leaders_data.added_leaders_cache.len(), 1);
    }
}
