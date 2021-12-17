use super::JormungandrRest;
use chain_impl_mockchain::block::BlockDate as ChainBlockDate;
use jormungandr_lib::interfaces::BlockDate;
use std::str::FromStr;

pub fn wait_for_epoch(epoch: u32, rest: JormungandrRest) {
    wait_for_date(ChainBlockDate { epoch, slot_id: 0 }.into(), rest)
}

pub fn wait_for_date(target_block_date: BlockDate, mut rest: JormungandrRest) {
    let settings = rest.settings().unwrap();
    while is_it_due(get_current_date(&mut rest), target_block_date) {
        std::thread::sleep(std::time::Duration::from_secs(settings.slot_duration));
    }
}

fn is_it_due(current_block_date: BlockDate, target_block_date: BlockDate) -> bool {
    println!(
        "waiting for block date : {}.{}/{}.{}",
        current_block_date.epoch(),
        current_block_date.slot(),
        target_block_date.epoch(),
        target_block_date.slot()
    );
    current_block_date < target_block_date
}

pub fn get_current_date(rest: &mut JormungandrRest) -> BlockDate {
    BlockDate::from_str(
        rest.stats()
            .unwrap()
            .stats
            .unwrap()
            .last_block_date
            .unwrap()
            .as_ref(),
    )
    .unwrap()
}
