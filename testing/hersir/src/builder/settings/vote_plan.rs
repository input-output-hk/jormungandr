use chain_impl_mockchain::certificate::VotePlanId;
use jormungandr_lib::interfaces::VotePlan;
use thor::wallet::committee::CommitteeDataManager;

#[derive(Debug, Clone)]
pub enum VotePlanSettings {
    Public(VotePlan),
    Private {
        keys: CommitteeDataManager,
        vote_plan: VotePlan,
    },
}

impl VotePlanSettings {
    pub fn vote_plan(&self) -> VotePlan {
        match self {
            Self::Public(vote_plan) => vote_plan.clone(),
            Self::Private {
                keys: _keys,
                vote_plan,
            } => vote_plan.clone(),
        }
    }

    pub fn to_id(&self) -> VotePlanId {
        let vote_plan: chain_impl_mockchain::certificate::VotePlan = self.vote_plan().into();
        vote_plan.to_id()
    }

    pub fn from_public_vote_plan(vote_plan: VotePlan) -> Self {
        Self::Public(vote_plan)
    }
}
