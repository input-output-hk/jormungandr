use crate::testing::node::explorer::Explorer;
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
