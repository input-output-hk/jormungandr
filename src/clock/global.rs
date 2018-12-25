use std::time::{Duration, SystemTime};

/// This represent (in spirit) the agreed time of the whole network
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlobalTime(SystemTime);

impl GlobalTime {
    pub fn now() -> GlobalTime {
        let gtime = SystemTime::now();
        GlobalTime(gtime)
    }

    pub fn differential(&self, earlier: GlobalTime) -> Duration {
        match self.0.duration_since(earlier.0) {
            Ok(duration) => duration,
            Err(e) => e.duration(),
        }
    }
}

/// This is absolute time the blockchain starts expressed in system time.
///
/// This is effectively T0 for the blockchain
pub struct BlockchainStart(GlobalTime);

/// Current time expressed in the number of seconds elapsed since the blockchain start time.
///
/// only 68 years available :)
pub struct BlockchainTime(u32);
