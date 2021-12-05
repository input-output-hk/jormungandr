use crate::blockcfg::{
    BlockDate, HeaderBft, HeaderBftBuilder, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
    HeaderSetConsensusSignature,
};
use chain_impl_mockchain::leadership::{Leader, LeaderOutput, Leadership};
use chain_time::Epoch;
use std::sync::Arc;

#[derive(Clone)]
pub struct Enclave {
    leader_data: Arc<Option<Leader>>,
}

pub struct LeaderEvent {
    pub date: BlockDate,
    pub output: LeaderOutput,
}

pub struct Schedule {
    enclave: Arc<Enclave>,
    leadership: Arc<Leadership>,
    current_slot: u32,
    stop_at_slot: u32,
    current_slot_data: Vec<LeaderEvent>,
}

impl Enclave {
    pub fn new(leader_data: Option<Leader>) -> Self {
        Enclave {
            leader_data: Arc::new(leader_data),
        }
    }

    pub fn create_header_genesis_praos(
        &self,
        header_builder: HeaderGenesisPraosBuilder<HeaderSetConsensusSignature>,
    ) -> Option<HeaderGenesisPraos> {
        let leader = self
            .leader_data
            .as_ref()
            .as_ref()?
            .genesis_leader
            .as_ref()?;
        let data = header_builder.get_authenticated_data();
        let signature = leader.sig_key.sign_slice(data);
        Some(header_builder.set_signature(signature.into()))
    }

    pub fn create_header_bft(
        &self,
        header_builder: HeaderBftBuilder<HeaderSetConsensusSignature>,
    ) -> Option<HeaderBft> {
        let leader = self.leader_data.as_ref().as_ref()?.bft_leader.as_ref()?;
        let data = header_builder.get_authenticated_data();
        let signature = leader.sig_key.sign_slice(data);
        Some(header_builder.set_signature(signature.into()))
    }
}

impl Schedule {
    pub fn new(
        enclave: Arc<Enclave>,
        leadership: Arc<Leadership>,
        slot_start: u32,
        nb_slots: u32,
    ) -> Self {
        let stop_at_slot = slot_start + nb_slots;

        Self {
            enclave,
            leadership,
            current_slot: slot_start,
            stop_at_slot,
            current_slot_data: Vec::new(),
        }
    }

    fn fill(&mut self) {
        let leader = if let Some(leader) = self.enclave.leader_data.as_ref() {
            leader
        } else {
            return;
        };

        if !self.current_slot_data.is_empty() {
            return;
        }

        while self.current_slot < self.stop_at_slot && self.current_slot_data.is_empty() {
            let date = self.leadership.date_at_slot(self.current_slot);
            match self.leadership.is_leader_for_date(leader, date) {
                LeaderOutput::None => (),
                leader_output => self.current_slot_data.push(LeaderEvent {
                    date,
                    output: leader_output,
                }),
            }

            self.current_slot += 1;
        }
    }

    pub fn next_event(&mut self) -> Option<LeaderEvent> {
        self.fill();
        self.current_slot_data.pop()
    }

    pub fn peek(&mut self) -> Option<&LeaderEvent> {
        self.fill();
        self.current_slot_data.last()
    }

    pub fn epoch(&self) -> Epoch {
        Epoch(self.leadership.epoch())
    }
}
