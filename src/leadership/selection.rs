use super::super::{
    secure::NodePublic,
    settings::{BftLeader, Consensus},
};

#[derive(PartialEq, Eq)]
pub enum IsLeading {
    Yes,
    No,
}

impl From<bool> for IsLeading {
    fn from(b: bool) -> Self {
        if b {
            IsLeading::Yes
        } else {
            IsLeading::No
        }
    }
}

pub trait BlockLeaderSelection {
    type DecisionParams;
    fn can_lead(&self) -> IsLeading;
    fn is_leader(&self, dp: Self::DecisionParams) -> IsLeading;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BftRoundRobinIndex(usize);

/// The BFT Leader selection is based on a round robin of the expected leaders
#[derive(Debug)]
pub struct BftLeaderSelection<LeaderId> {
    my: Option<BftRoundRobinIndex>,
    leaders: Vec<LeaderId>,
}

impl<LeaderId: Eq> BftLeaderSelection<LeaderId> {
    /// Create a new BFT leadership
    pub fn new(me: LeaderId, leaders: Vec<LeaderId>) -> Option<Self> {
        if leaders.len() == 0 {
            return None;
        }

        let pos = leaders
            .iter()
            .position(|x| x == &me)
            .map(BftRoundRobinIndex);
        Some(BftLeaderSelection {
            my: pos,
            leaders: leaders,
        })
    }

    pub fn number_of_leaders(&self) -> usize {
        self.leaders.len()
    }

    /// get the party leader id elected for a given slot
    pub fn get_leader_at(&self, slotid: u64) -> &LeaderId {
        let max = self.leaders.len();
        let ofs = (slotid % max as u64) as usize;
        &self.leaders[ofs]
    }

    /// check if this party is elected for a given slot
    pub fn am_leader_at(&self, slotid: u64) -> IsLeading {
        match self.my {
            None => IsLeading::No,
            Some(my_index) => {
                let max = self.leaders.len();
                let slot_offset = BftRoundRobinIndex((slotid % max as u64) as usize);
                (slot_offset == my_index).into()
            }
        }
    }
}

impl<LeaderId: Eq> BlockLeaderSelection for BftLeaderSelection<LeaderId> {
    type DecisionParams = (LeaderId, u64);

    fn can_lead(&self) -> IsLeading {
        self.my.map_or(false, |_| true).into()
    }

    fn is_leader(&self, dp: Self::DecisionParams) -> IsLeading {
        (&dp.0 == self.get_leader_at(dp.1)).into()
    }
}

pub enum Selection {
    Bft(BftLeaderSelection<BftLeader>),
    Genesis,
}

pub fn can_lead(selection: &Selection) -> IsLeading {
    match selection {
        Selection::Bft(sel) => sel.can_lead(),
        Selection::Genesis => IsLeading::No,
    }
}

pub fn prepare(public: &NodePublic, consensus: &Consensus) -> Option<Selection> {
    match consensus {
        Consensus::Bft(bft) => {
            let p = &BftLeader(public.block_publickey);
            BftLeaderSelection::new(p.clone(), bft.leaders.clone()).map(Selection::Bft)
        }
        Consensus::Genesis => Some(Selection::Genesis),
    }
}

pub fn test(selection: &Selection, flat_slotid: u64) -> IsLeading {
    match selection {
        Selection::Bft(sel) => sel.am_leader_at(flat_slotid),
        // TODO: genesis never elected for now
        Selection::Genesis => IsLeading::No,
    }
}
