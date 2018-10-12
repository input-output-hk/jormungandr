use std::time::SystemTime;

/// This represent (in spirit) the agreed time of the whole network
pub struct GlobalTime(SystemTime);

/// This is absolute time the blockchain starts expressed in system time.
///
/// This is effectively T0 for the blockchain
pub struct BlockchainStart(GlobalTime);

/// Current time expressed in the number of seconds elapsed since the blockchain start time.
pub struct BlockchainTime(u32);

pub fn get_time_now() -> GlobalTime {
    let gtime = SystemTime::now();
    GlobalTime(gtime)
}

