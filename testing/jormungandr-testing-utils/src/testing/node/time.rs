use crate::testing::node::explorer::Explorer;
use chain_impl_mockchain::block::ChainLength;
use jormungandr_lib::interfaces::BlockDate;

pub fn wait_for_epoch(epoch_id: u64, mut explorer: Explorer) {
    explorer.enable_logs();
    while explorer
        .last_block()
        .unwrap()
        .data
        .unwrap()
        .tip
        .block
        .date
        .epoch
        .id
        .parse::<u64>()
        .unwrap()
        < epoch_id
    {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

pub fn wait_for_date(target_block_date: BlockDate, mut explorer: Explorer) {
    explorer.enable_logs();

    loop {
        let current_block_date = explorer.last_block().unwrap().data.unwrap().tip.block.date;

        let epoch = current_block_date.epoch.id.parse::<u32>().unwrap();
        let slot_id = current_block_date.slot.parse::<u32>().unwrap();

        let current_block_date = BlockDate::new(epoch, slot_id);

        if target_block_date <= current_block_date {
            return;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

pub fn wait_n_blocks(start: ChainLength, n: u32, explorer: &Explorer) {
    loop {
        let current = explorer
            .last_block()
            .unwrap()
            .data
            .unwrap()
            .tip
            .block
            .chain_length;

        let current: u32 = current.parse().unwrap();

        if u32::from(start) + n <= current {
            return;
        }

        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}
