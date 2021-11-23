pub mod disruption;
pub mod fragment_propagation;

const PASSIVE: &str = "Passive";
const LEADER: &str = "Leader";
const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";
const LEADER_3: &str = "Leader3";
const LEADER_4: &str = "Leader4";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";
const CLARICE: &str = "CLARICE";
const DAVID: &str = "DAVID";

pub use disruption::*;
pub use fragment_propagation::*;
