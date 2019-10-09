use crate::{
    testing::data::StakePool,
    block::{BlockDate,ChainLength,BlockBuilder,Block, Contents, Header},
    key::Hash
};
use chain_crypto::vrf_evaluate_and_prove;
use chain_time::TimeEra;

pub struct GenesisPraosBlockBuilder{
    date: Option<BlockDate>,
    chain_length: Option<ChainLength>,
    parent_id: Option<Hash>
}

impl GenesisPraosBlockBuilder {
    pub fn new() -> Self {
        GenesisPraosBlockBuilder{
            date: None,
            chain_length: None,
            parent_id: None
        }
    }

    pub fn with_parent(&mut self, parent: &Header) -> &mut Self  {
        self.with_parent_id(parent.hash());
        self.with_date(*parent.block_date());
        self.with_chain_length(parent.chain_length());
        self
    }

    pub fn with_parent_id(&mut self, parent_id: Hash) -> &mut Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn with_date(&mut self, date: BlockDate) -> &mut Self  {
        self.date = Some(date);
        self
    }

    pub fn with_chain_length(&mut self, chain_length: ChainLength) -> &mut Self  {
        self.chain_length = Some(chain_length);
        self
    }

    pub fn build(&self, stake_pool: &StakePool, time_era: &TimeEra) -> Block {
        if self.date.is_none() || self.chain_length.is_none()|| self.parent_id.is_none() {
            panic!("date,chain_length or hash is not set");
        }

        let mut rng = rand_os::OsRng::new().unwrap();
        let vrf_proof =
            vrf_evaluate_and_prove(stake_pool.vrf().private_key(), &[0, 1, 2, 3], &mut rng);
        let mut block_builder = BlockBuilder::new(Contents::empty());
        block_builder.date(self.date.unwrap().next(time_era));
        block_builder.chain_length(self.chain_length.unwrap().increase());
        block_builder.parent(self.parent_id.unwrap());
        block_builder.make_genesis_praos_block(
            &stake_pool.id(),
            stake_pool.kes().private_key(),
            vrf_proof,
        )
    }

}
