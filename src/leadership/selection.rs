use super::super::{
    secure::NodePublic, settings::{BftLeader, Consensus},
};

pub enum Selection {
    Bft(usize, Option<usize>),
    Genesis,
}

pub fn prepare(public: &NodePublic, consensus: &Consensus) -> Selection {
    match consensus {
        Consensus::Bft(bft) => {
            let p = &BftLeader(public.block_publickey);
            let pos = bft.leaders.iter().position(|x| x == p);
            Selection::Bft(bft.leaders.len(), pos)
        }
        Consensus::Genesis => Selection::Genesis,
    }
}

pub fn test(selection: &Selection, slotid: u32) -> bool {
    match selection {
        Selection::Bft(nb_leaders, i) => {
            match i {
                None    => false,
                Some(i) => &(slotid as usize % nb_leaders) == i
            }
        },
        // TODO: genesis never elected for now
        Selection::Genesis => false,
    }
}
