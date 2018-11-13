pub mod process;

pub use self::process::leadership_task;

use super::secure::NodePublic;
use super::settings::{Consensus, BftLeader};

pub fn can_node_lead(public: &NodePublic, consensus: &Consensus) -> bool {
    match consensus {
        Consensus::Bft(bft) => {
            let p = BftLeader(public.block_publickey);
            let found = bft.leaders.contains(&p);
            println!("BFT our node can lead: {}", found);
            found
        },
        Consensus::Genesis => true,
    }
}
