use crate::scenario::repository::MeasurementThresholds;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum SyncWaitParams {
    Standard {
        no_of_nodes: u64,
        longest_path_length: u64,
    },
    WithDisruption {
        no_of_nodes: u64,
        restart_coeff: u64,
    },
    ZeroWait,
}

impl SyncWaitParams {
    pub fn network_size(no_of_nodes: u64, longest_path_length: u64) -> Self {
        Self::Standard {
            no_of_nodes,
            longest_path_length,
        }
    }

    pub fn two_nodes() -> Self {
        Self::network_size(2, 2)
    }

    pub fn nodes_restart(no_of_nodes: u64) -> Self {
        Self::WithDisruption {
            no_of_nodes: no_of_nodes,
            restart_coeff: 30,
        }
    }

    fn calculate_wait_time(&self) -> u64 {
        match self {
            Self::Standard {
                no_of_nodes,
                longest_path_length,
            } => (no_of_nodes + longest_path_length * 2) * 2,
            Self::WithDisruption {
                no_of_nodes,
                restart_coeff,
            } => return *no_of_nodes * restart_coeff * 2,
            Self::ZeroWait => 0,
        }
    }

    pub fn wait_time(&self) -> Duration {
        Duration::from_secs(self.calculate_wait_time())
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.calculate_wait_time() * 2)
    }
}

impl Into<MeasurementThresholds> for SyncWaitParams {
    fn into(self) -> MeasurementThresholds {
        match self {
            SyncWaitParams::WithDisruption {
                no_of_nodes: _,
                restart_coeff: _,
            } => unimplemented!(),
            SyncWaitParams::Standard {
                no_of_nodes,
                longest_path_length,
            } => {
                let green = no_of_nodes;
                let yellow = no_of_nodes + longest_path_length;
                let red = no_of_nodes + longest_path_length * 2;
                let timeout = no_of_nodes * 2 + longest_path_length * 2;

                MeasurementThresholds::new(green, yellow, red, timeout)
            }
            SyncWaitParams::ZeroWait => MeasurementThresholds::new(0, 0, 0, 0),
        }
    }
}
