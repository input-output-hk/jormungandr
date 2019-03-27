use chain_core::property::HasMessages as _;
pub use network_core::gossip::Gossip;

pub use chain_impl_mockchain::{
    block::{Block, BlockBuilder, BlockDate, ChainLength, Header, HeaderHash},
    config::{entity_from, Block0Date, ConfigParam},
    leadership::{Leader, LeaderId, LeaderOutput, Leadership},
    ledger::{Ledger, LedgerParameters, LedgerStaticParameters},
    message::{InitialEnts, Message, MessageId},
    multiverse::Multiverse,
};

fn block_0_get_initial(block: &Block) -> &InitialEnts {
    for message in block.messages() {
        if let Message::Initial(init) = message {
            return init;
        }
    }

    panic!("Invalid Block0: missing the initial blockchain settings");
}

pub fn block_0_get_start_time(block: &Block) -> std::time::SystemTime {
    let ents = block_0_get_initial(block);

    for (tag, payload) in ents.iter() {
        match tag {
            &<Block0Date as ConfigParam>::TAG => {
                let Block0Date(time_since_epoch) = entity_from(*tag, &payload).unwrap();
                return std::time::SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(time_since_epoch);
            }
            _ => {}
        }
    }

    panic!("Invalid Block0: missing the start time of the blockchain in the settings");
}
