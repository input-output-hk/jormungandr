use super::super::data::last_block;
use crate::testing::node::explorer::BlockDate;
use chain_impl_mockchain::block::BlockDate as LibBlockDate;
use graphql_client::Response;

#[derive(Debug)]
pub struct LastBlockResponse {
    data: Response<last_block::ResponseData>,
}

impl LastBlockResponse {
    pub fn new(data: Response<last_block::ResponseData>) -> Self {
        Self { data }
    }

    pub fn rewards(&self) -> u64 {
        self.data
            .data
            .as_ref()
            .unwrap()
            .tip
            .block
            .treasury
            .as_ref()
            .unwrap()
            .rewards
            .parse::<u64>()
            .unwrap()
    }

    pub fn block_date(&self) -> BlockDate {
        let date = &self.data.data.as_ref().unwrap().tip.block.date;

        let block_date = LibBlockDate {
            epoch: date.epoch.id.parse().unwrap(),
            slot_id: date.slot.parse().unwrap(),
        };
        BlockDate::from(block_date)
    }
}
