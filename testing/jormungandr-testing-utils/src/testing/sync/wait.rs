use crate::testing::{Speed, Thresholds};
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
    pub fn large_network(no_of_nodes: u64) -> Self {
        Self::Standard {
            no_of_nodes: no_of_nodes * 4,
            longest_path_length: no_of_nodes * 4,
        }
    }

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
            no_of_nodes,
            restart_coeff: 30,
        }
    }

    fn calculate_wait_time(&self) -> u64 {
        match self {
            Self::Standard {
                no_of_nodes,
                longest_path_length,
            } => (no_of_nodes * 2 + longest_path_length * 2) * 2,
            Self::WithDisruption {
                no_of_nodes,
                restart_coeff,
            } => *no_of_nodes * restart_coeff * 2,
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

impl From<SyncWaitParams> for Thresholds<Speed> {
    fn from(params: SyncWaitParams) -> Thresholds<Speed> {
        let grace_coeff = 2;

        match params {
            SyncWaitParams::WithDisruption {
                no_of_nodes,
                restart_coeff,
            } => {
                let green = Duration::from_secs(no_of_nodes * restart_coeff) * grace_coeff;
                let yellow = Duration::from_secs(no_of_nodes * restart_coeff * 2) * grace_coeff;
                let red = Duration::from_secs(no_of_nodes * restart_coeff * 3) * grace_coeff;
                let timeout = Duration::from_secs(no_of_nodes * restart_coeff * 4) * grace_coeff;

                Thresholds::<Speed>::new(green.into(), yellow.into(), red.into(), timeout.into())
            }
            SyncWaitParams::Standard {
                no_of_nodes,
                longest_path_length,
            } => {
                let green = Duration::from_secs(no_of_nodes) * grace_coeff;
                let yellow = Duration::from_secs(no_of_nodes + longest_path_length) * grace_coeff;
                let red = Duration::from_secs(no_of_nodes + longest_path_length * 2) * grace_coeff;
                let timeout =
                    Duration::from_secs(no_of_nodes * 2 + longest_path_length * 2) * grace_coeff;

                Thresholds::<Speed>::new(green.into(), yellow.into(), red.into(), timeout.into())
            }
            SyncWaitParams::ZeroWait => {
                let duration = Duration::from_secs(0);
                Thresholds::<Speed>::new(
                    duration.clone().into(),
                    duration.clone().into(),
                    duration.clone().into(),
                    duration.clone().into(),
                )
            }
        }
    }
}
