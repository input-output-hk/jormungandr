use chain_impl_mockchain::{
    header::BlockDate,
    testing::{GenesisPraosBlockBuilder, StakePoolBuilder},
};
use chain_time::{Epoch, TimeEra};
use jormungandr_automation::jormungandr::grpc::server::MockBuilder;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let port: u16 = args[1].parse().unwrap();

    let mut mock_controller = MockBuilder::default().with_port(port).build();

    std::thread::sleep(std::time::Duration::from_secs(60));

    let stake_pool = StakePoolBuilder::new().build();
    let time_era = TimeEra::new(0u64.into(), Epoch(0u32), 30);

    let block = GenesisPraosBlockBuilder::new()
        .with_parent_id(mock_controller.genesis_hash())
        .with_date(BlockDate {
            epoch: 0,
            slot_id: 1,
        })
        .with_chain_length(1.into())
        .build(&stake_pool, &time_era);

    mock_controller.set_tip_block(&block);
    std::thread::sleep(std::time::Duration::from_secs(60));
    mock_controller.stop();
}
