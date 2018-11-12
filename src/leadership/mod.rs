pub mod process;

pub use self::process::leadership_task;

use super::secure::NodePublic;
use super::settings::Consensus;

pub fn can_node_lead(public: &NodePublic, consensus: &Consensus) -> bool {
    match consensus {
        Consensus::Bft(bft) => {
            false
        },
        Consensus::Genesis => true,
    }
}
