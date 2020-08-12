use crate::{Query, Schedule};
use blockchain::EpochInfo;
use chain_impl_mockchain::header::{
    HeaderBft, HeaderBftBuilder, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
    HeaderSetConsensusSignature, SlotId,
};
use chain_impl_mockchain::leadership::{Leader, LeaderOutput};
use std::sync::{Arc, Mutex};

pub struct InMemory(Arc<Mutex<Leader>>);

impl InMemory {
    pub fn new(leader: Leader) -> Self {
        Self(Arc::new(Mutex::new(leader)))
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
                epoch_info,
                reply,
            } => self.schedule(from, length, epoch_info, reply),
        }
    }

    /// calling this function may be a bit intensive, so instead it will
    /// call the result
    fn schedule(
        &self,
        from: SlotId,
        length: usize,
        epoch_info: Arc<EpochInfo>,
        reply: Box<dyn FnOnce(Vec<Schedule>) + Send + 'static>,
    ) {
        let inner = self.0.clone();
        std::thread::spawn(move || {
            let leader = inner.lock().ok()?;
            let mut schedules = Vec::with_capacity(std::cmp::min(128, length));
            for slot_id in from..(from + length as u32) {
                let date = epoch_info.epoch_leadership_schedule().date_at_slot(slot_id);
                match epoch_info
                    .epoch_leadership_schedule()
                    .is_leader_for_date(&leader, date)
                {
                    Ok(LeaderOutput::None) => continue,
                    Ok(output) => {
                        let schedule = Schedule { output, date };
                        schedules.push(schedule);
                    }
                    Err(_) => {
                        // For now silently ignore errors
                    }
                }
            }

            reply(schedules);
            Some(())
        });
    }

    fn sign_bft(
        &self,
        header_builder: HeaderBftBuilder<HeaderSetConsensusSignature>,
        reply: Box<dyn FnOnce(Option<HeaderBft>) + Send + 'static>,
    ) {
        let inner = self.0.clone();
        std::thread::spawn(move || {
            if let Ok(inner) = inner.lock() {
                let s = if let Some(bft_leader) = &inner.bft_leader {
                    let data = header_builder.get_authenticated_data();
                    let signature = bft_leader.sig_key.sign_slice(data);
                    Some(header_builder.set_signature(signature.into()))
                } else {
                    None
                };
                reply(s)
            } else {
                reply(None)
            }
        });
    }

    fn sign_genesis_praos(
        &mut self,
        header_builder: HeaderGenesisPraosBuilder<HeaderSetConsensusSignature>,
        reply: Box<dyn FnOnce(Option<HeaderGenesisPraos>) + Send + 'static>,
    ) {
        let inner = self.0.clone();
        std::thread::spawn(move || {
            if let Ok(inner) = inner.lock() {
                let s = if let Some(genesis_leader) = &inner.genesis_leader {
                    let data = header_builder.get_authenticated_data();
                    let signature = genesis_leader.sig_key.sign_slice(data);
                    Some(header_builder.set_signature(signature.into()))
                } else {
                    None
                };
                reply(s)
            } else {
                reply(None)
            }
        });
    }
}
