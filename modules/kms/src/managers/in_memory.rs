use crate::{Query, Schedule};
use chain_impl_mockchain::header::{
    HeaderBft, HeaderBftBuilder, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
    HeaderSetConsensusSignature, SlotId,
};
use chain_impl_mockchain::leadership::{Leader, LeaderOutput, Leadership};
use std::sync::Arc;

pub struct InMemory(Leader);

impl InMemory {
    pub fn new(leader: Leader) -> Self {
        Self(leader)
    }

    pub(crate) fn query(&mut self, query: Query) {
        match query {
            Query::SignBft {
                header_builder,
                reply,
            } => self.sign_bft(header_builder, reply),
            Query::SignGenesisPraos {
                header_builder,
                reply,
            } => self.sign_genesis_praos(header_builder, reply),
            Query::Schedules {
                from,
                length,
                leadership,
                reply,
            } => self.schedule(from, length, leadership, reply),
        }
    }

    fn schedule(
        &self,
        from: SlotId,
        length: usize,
        leadership: Arc<Leadership>,
        reply: Box<dyn FnOnce(Vec<Schedule>)>,
    ) {
        let mut schedules = Vec::with_capacity(std::cmp::min(128, length));
        for slot_id in from..(from + length as u32) {
            let date = leadership.date_at_slot(slot_id);
            match leadership.is_leader_for_date(&self.0, date) {
                Ok(LeaderOutput::None) => continue,
                Ok(output) => {
                    let schedule = Schedule { output, slot_id };
                    schedules.push(schedule);
                }
                Err(_) => {
                    // For now silently ignore errors
                }
            }
        }

        reply(schedules)
    }

    fn sign_bft(
        &self,
        header_builder: HeaderBftBuilder<HeaderSetConsensusSignature>,
        reply: Box<dyn FnOnce(Option<HeaderBft>)>,
    ) {
        let s = if let Some(bft_leader) = &self.0.bft_leader {
            let data = header_builder.get_authenticated_data();
            let signature = bft_leader.sig_key.sign_slice(data);
            Some(header_builder.set_signature(signature.into()))
        } else {
            None
        };
        reply(s)
    }

    fn sign_genesis_praos(
        &mut self,
        header_builder: HeaderGenesisPraosBuilder<HeaderSetConsensusSignature>,
        reply: Box<dyn FnOnce(Option<HeaderGenesisPraos>)>,
    ) {
        let s = if let Some(genesis_leader) = &self.0.genesis_leader {
            let data = header_builder.get_authenticated_data();
            let signature = genesis_leader.sig_key.sign_slice(data);
            Some(header_builder.set_signature(signature.into()))
        } else {
            None
        };
        reply(s)
    }
}
