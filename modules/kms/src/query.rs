use chain_impl_mockchain::{
    header::{
        HeaderBft, HeaderBftBuilder, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
        HeaderSetConsensusSignature, SlotId,
    },
    leadership::{LeaderOutput, Leadership},
};
use std::sync::Arc;

pub struct Schedule {
    pub slot_id: SlotId,
    pub output: LeaderOutput,
}

pub enum Query {
    SignBft {
        header_builder: HeaderBftBuilder<HeaderSetConsensusSignature>,
        reply: Box<dyn FnOnce(Option<HeaderBft>)>,
    },
    SignGenesisPraos {
        header_builder: HeaderGenesisPraosBuilder<HeaderSetConsensusSignature>,
        reply: Box<dyn FnOnce(Option<HeaderGenesisPraos>)>,
    },
    Schedules {
        from: SlotId,
        length: usize,
        leadership: Arc<Leadership>,
        reply: Box<dyn FnOnce(Vec<Schedule>)>,
    },
}
