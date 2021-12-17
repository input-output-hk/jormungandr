pub mod big;
pub mod desync;
pub mod disruption;
#[cfg(feature = "soak")]
pub mod soak;

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";

const PASSIVE: &str = "Passive";
const LEADER: &str = "Leader";
const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";
const LEADER_3: &str = "Leader3";
const LEADER_4: &str = "Leader4";
const LEADER_5: &str = "Leader5";
