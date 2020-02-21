use jormungandr_lib::testing::Thresholds;
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
            } => (no_of_nodes * 2 + longest_path_length * 2) * 2,
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

impl Into<Thresholds<Duration>> for SyncWaitParams {
    fn into(self) -> Thresholds<Duration> {
        match self {
            SyncWaitParams::WithDisruption {
                no_of_nodes,
                restart_coeff,
            } => {
                let green = Duration::from_secs(no_of_nodes * restart_coeff);
                let yellow = Duration::from_secs(no_of_nodes * restart_coeff * 2);
                let red = Duration::from_secs(no_of_nodes * restart_coeff * 3);
                let timeout = Duration::from_secs(no_of_nodes * restart_coeff * 4);

                Thresholds::<Duration>::new(green, yellow, red, timeout)
            }
            SyncWaitParams::Standard {
                no_of_nodes,
                longest_path_length,
            } => {
                let green = Duration::from_secs(no_of_nodes);
                let yellow = Duration::from_secs(no_of_nodes + longest_path_length);
                let red = Duration::from_secs(no_of_nodes + longest_path_length * 2);
                let timeout = Duration::from_secs(no_of_nodes * 2 + longest_path_length * 2);

                Thresholds::<Duration>::new(green, yellow, red, timeout)
            }
            SyncWaitParams::ZeroWait => {
                let duration = Duration::from_secs(0);
                Thresholds::<Duration>::new(
                    duration.clone(),
                    duration.clone(),
                    duration.clone(),
                    duration.clone(),
                )
            }
        }
    }
}
