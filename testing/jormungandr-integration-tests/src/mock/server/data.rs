use super::ProtocolVersion;
use chain_impl_mockchain::{
    block::{BlockDate, Header},
    key::Hash,
    testing::{GenesisPraosBlockBuilder, StakePoolBuilder},
};
use chain_time::{Epoch, TimeEra};

pub struct MockServerData {
    genesis_hash: Hash,
    tip: Header,
    protocol: ProtocolVersion,
}

impl MockServerData {
    pub fn new(genesis_hash: Hash, tip: Header, protocol: ProtocolVersion) -> Self {
        Self {
            genesis_hash,
            tip,
            protocol,
        }
    }

    pub fn genesis_hash(&self) -> &Hash {
        &self.genesis_hash
    }

    pub fn tip(&self) -> &Header {
        &self.tip
    }

    pub fn protocol(&self) -> &ProtocolVersion {
        &self.protocol
    }

    pub fn genesis_hash_mut(&mut self) -> &mut Hash {
        &mut self.genesis_hash
    }

    pub fn tip_mut(&mut self) -> &mut Header {
        &mut self.tip
    }

    pub fn protocol_mut(&mut self) -> &mut ProtocolVersion {
        &mut self.protocol
    }
}

pub fn header(slots_per_epochs: u32, parent_id: &Hash) -> Header {
    let stake_pool = StakePoolBuilder::new().build();

    let time_era = TimeEra::new(0u64.into(), Epoch(0u32), slots_per_epochs);

    let block = GenesisPraosBlockBuilder::new()
        .with_parent_id(*parent_id)
        .with_date(BlockDate {
            epoch: 0,
            slot_id: 1,
        })
        .with_chain_length(1.into())
        .build(&stake_pool, &time_era);
    block.header
}
