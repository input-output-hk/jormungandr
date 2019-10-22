use crate::blockcfg::{
    BlockDate, HeaderBft, HeaderBftBuilder, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
    HeaderSetConsensusSignature, SlotId,
};
use chain_impl_mockchain::leadership::{Leader, LeaderOutput, Leadership};
use jormungandr_lib::interfaces::EnclaveLeaderId as LeaderId;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Enclave {
    leaders: Arc<RwLock<BTreeMap<LeaderId, Leader>>>,
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
        }
    }

    pub fn from_vec(leaders: Vec<Leader>) -> Self {
        let e = Self::new();
        for leader in leaders {
            e.add_leader(leader);
        }
        e
    }

    pub fn get_leaderids(&self) -> Vec<LeaderId> {
        let leaders = self.leaders.read().unwrap();
        leaders.keys().map(|v| v.clone()).collect()
    }

    pub fn add_leader(&self, leader: Leader) -> LeaderId {
        let mut leaders = self.leaders.write().unwrap();
        let next_leader_id = get_maximum_id(&leaders).next();
        // This panic case should never happens in practice, as this structure is
        // not supposed to be shared between thread.
        match leaders.insert(next_leader_id, leader) {
            None => (),
            Some(_) => panic!("enclave leader failed : duplicated value race"),
        };
        next_leader_id
    }

    pub fn remove_leader(&self, leader_id: LeaderId) -> bool {
        let mut leaders = self.leaders.write().unwrap();
        leaders.remove(&leader_id).is_some()
    }

    // temporary method
    pub fn leadership_evaluate1(
        &self,
        leadership: &Leadership,
        leader_id: &LeaderId,
        slot: SlotId,
    ) -> Option<LeaderEvent> {
        let leaders = self.leaders.read().unwrap();
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

    pub fn leadership_evaluate(
        &self,
        leadership: &Leadership,
        slot_start: u32,
        nb_slots: u32,
    ) -> Vec<LeaderEvent> {
        let leaders = self.leaders.read().unwrap();
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

    pub fn create_header_genesis_praos(
        &self,
        header_builder: HeaderGenesisPraosBuilder<HeaderSetConsensusSignature>,
        id: LeaderId,
    ) -> Option<HeaderGenesisPraos> {
        let leaders = self.leaders.read().unwrap();
        let leader = leaders.get(&id)?;
        if let Some(genesis_leader) = &leader.genesis_leader {
            let data = header_builder.get_authenticated_data();
            let signature = genesis_leader.sig_key.sign_slice(data);
            Some(header_builder.set_signature(signature.into()))
        } else {
            None
        }
    }

    pub fn create_header_bft(
        &self,
        header_builder: HeaderBftBuilder<HeaderSetConsensusSignature>,
        id: LeaderId,
    ) -> Option<HeaderBft> {
        let leaders = self.leaders.read().unwrap();
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
