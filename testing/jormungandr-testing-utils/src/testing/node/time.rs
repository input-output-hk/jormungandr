use crate::testing::node::explorer::Explorer;

pub fn wait_for_epoch(epoch_id: u64, mut explorer: Explorer) {
    explorer.enable_logs();
    while explorer
        .status()
        .unwrap()
        .data
        .unwrap()
        .status
        .latest_block
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
