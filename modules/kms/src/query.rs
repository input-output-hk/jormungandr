use blockchain::EpochInfo;
use chain_impl_mockchain::{
    header::{
        BlockDate, HeaderBft, HeaderBftBuilder, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
        HeaderSetConsensusSignature, SlotId,
    },
    leadership::LeaderOutput,
};
use std::sync::Arc;

pub struct Schedule {
    pub date: BlockDate,
    pub output: LeaderOutput,
}

pub enum Query {
    SignBft {
        header_builder: HeaderBftBuilder<HeaderSetConsensusSignature>,
        reply: Box<dyn FnOnce(Option<HeaderBft>) + Send + 'static>,
    },
    SignGenesisPraos {
        header_builder: HeaderGenesisPraosBuilder<HeaderSetConsensusSignature>,
        reply: Box<dyn FnOnce(Option<HeaderGenesisPraos>) + Send + 'static>,
    },
    Schedules {
        from: SlotId,
        length: usize,
        epoch_info: Arc<EpochInfo>,
        reply: Box<dyn FnOnce(Vec<Schedule>) + Send + 'static>,
    },
}
