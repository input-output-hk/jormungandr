use crate::blockcfg::{
    BlockDate, HeaderBft, HeaderBftBuilder, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
    HeaderSetConsensusSignature, SlotId,
};
use chain_impl_mockchain::leadership::{Leader, LeaderOutput, Leadership};
use jormungandr_lib::interfaces::EnclaveLeaderId as LeaderId;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tokio02::sync::RwLock;

#[derive(Clone)]
pub struct Enclave {
    leaders: Arc<RwLock<BTreeMap<LeaderId, Leader>>>,
    added_cache: Arc<RwLock<HashMap<String, LeaderId>>>,
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
            leaders: Arc::new(RwLock::new(BTreeMap::new())),
            added_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn from_vec(leaders: Vec<Leader>) -> Self {
        let e = Self::new();
        for leader in leaders {
            e.add_leader(leader).await;
        }
        e
    }

    pub async fn get_leader_id_if_present(&self, leader: &Leader) -> Option<LeaderId> {
        match &leader.bft_leader {
            Some(l) => {
                let cache = self.added_cache.read().await;
                cache.get(&l.sig_key.to_public().to_string()).cloned()
            }
            None => None,
        }
    }
    pub async fn get_leaderids(&self) -> Vec<LeaderId> {
        let leaders = self.leaders.read().await;
        leaders.keys().cloned().collect()
    }

    pub async fn add_leader(&self, leader: Leader) -> LeaderId {
        match self.get_leader_id_if_present(&leader).await {
            Some(id) => return id,
            None => {}
        }
        let mut leaders = self.leaders.write().await;
        let next_leader_id = get_maximum_id(&leaders).next();
        let mut cache = self.added_cache.write().await;
        leader
            .bft_leader
            .as_ref()
            .map(|l| cache.insert(l.sig_key.to_public().to_string(), next_leader_id));
        // This panic case should never happens in practice, as this structure is
        // not supposed to be shared between thread.
        match leaders.insert(next_leader_id, leader) {
            None => (),
            Some(_) => panic!("enclave leader failed : duplicated value race"),
        };
        next_leader_id
    }

    pub async fn remove_leader(&self, leader_id: LeaderId) -> bool {
        let mut leaders = self.leaders.write().await;
        leaders.remove(&leader_id).is_some()
    }

    // temporary method
    pub async fn leadership_evaluate1(
        &self,
        leadership: &Leadership,
        leader_id: &LeaderId,
        slot: SlotId,
    ) -> Option<LeaderEvent> {
        let leaders = self.leaders.read().await;
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
        let leaders = self.leaders.read().await;
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
        let leaders = self.leaders.read().await;
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
        let leaders = self.leaders.read().await;
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
    use chain_crypto::{Ed25519, SecretKey};
    use chain_impl_mockchain::leadership::BftLeader;
    use rand_core;
    use tokio02 as tokio;

    #[tokio::test]
    async fn enclave_add_different_leaders() {
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
        assert_eq!(enclave.leaders.read().await.len(), 2);
        assert_eq!(enclave.added_cache.read().await.len(), 2);
    }

    #[tokio::test]
    async fn enclave_add_duplicated_leaders() {
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
        assert_eq!(enclave.leaders.read().await.len(), 1);
        assert_eq!(enclave.added_cache.read().await.len(), 1);
    }
}
