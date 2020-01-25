pub mod disruption;
pub mod soak;
pub mod sync;

use crate::{scenario::repository::MeasurementThresholds, test::utils::SyncWaitParams};

const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";
const LEADER_3: &str = "Leader3";
const LEADER_4: &str = "Leader4";
const LEADER_5: &str = "Leader5";
const LEADER_6: &str = "Leader6";
const LEADER_7: &str = "Leader7";

pub fn sync_threshold(sync_wait_params: SyncWaitParams) -> MeasurementThresholds {
    let no_of_nodes = sync_wait_params.no_of_nodes;
    let longest_path_length = sync_wait_params.longest_path_length;

    let green = no_of_nodes;
    let yellow = no_of_nodes + longest_path_length;
    let red = no_of_nodes + longest_path_length * 2;
    let timeout = no_of_nodes * 2 + longest_path_length * 2;

    MeasurementThresholds::new(green, yellow, red, timeout)
}
