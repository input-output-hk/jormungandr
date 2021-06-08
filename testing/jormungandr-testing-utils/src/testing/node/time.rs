use super::JormungandrRest;
use jormungandr_lib::interfaces::BlockDate;
use std::str::FromStr;

pub fn wait_for_epoch(target_epoch_id: u32, mut rest: JormungandrRest) {
    rest.enable_logger();

    while get_current_date(&mut rest).epoch() < target_epoch_id {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

pub fn wait_for_date(target_block_date: BlockDate, mut rest: JormungandrRest) {
    rest.enable_logger();

    while get_current_date(&mut rest) < target_block_date {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn get_current_date(rest: &mut JormungandrRest) -> BlockDate {
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
