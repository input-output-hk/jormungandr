use chain_core::property::BlockDate as _;
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{Proposal, Proposals, PushProposal, VoteAction, VotePlan},
    ledger::governance::ParametersGovernanceAction,
    testing::VoteTestGen,
    value::Value,
    vote::{Options, PayloadType},
};
mod builder;
pub use builder::VotePlanBuilder;

pub fn proposal_with_3_options(rewards_increase: u64) -> Proposal {
    let action = VoteAction::Parameters {
        action: ParametersGovernanceAction::RewardAdd {
            value: Value(rewards_increase),
        },
    };

    Proposal::new(
        VoteTestGen::external_proposal_id(),
        Options::new_length(3).unwrap(),
        action,
    )
}

pub fn offchain_proposal() -> Proposal {
    Proposal::new(
        VoteTestGen::external_proposal_id(),
        Options::new_length(3).unwrap(),
        VoteAction::OffChain,
    )
}

pub fn proposals(rewards_increase: u64) -> Proposals {
    let mut proposals = Proposals::new();
    for _ in 0..3 {
        assert_eq!(
            PushProposal::Success,
            proposals.push(proposal_with_3_options(rewards_increase)),
            "generate_proposal method is only for correct data preparation"
        );
    }
    proposals
}

pub trait VotePlanExtension {
    fn as_json(&self) -> json::JsonValue;
    fn as_json_str(&self) -> String;
}

impl VotePlanExtension for VotePlan {
    fn as_json(&self) -> json::JsonValue {
        let mut data = json::JsonValue::new_object();

        let payload = match self.payload_type() {
            PayloadType::Public => "public",
            PayloadType::Private => "private",
        };

        data["payload_type"] = json::JsonValue::String(payload.to_owned());

        let mut vote_start = json::JsonValue::new_object();
        vote_start["epoch"] = self.vote_start().epoch.into();
        vote_start["slot_id"] = self.vote_start().slot_id.into();

        data["vote_start"] = vote_start;

        let mut vote_end = json::JsonValue::new_object();
        vote_end["epoch"] = self.vote_end().epoch.into();
        vote_end["slot_id"] = self.vote_end().slot_id.into();

        data["vote_end"] = vote_end;

        let mut committee_end = json::JsonValue::new_object();
        committee_end["epoch"] = self.committee_end().epoch.into();
        committee_end["slot_id"] = self.committee_end().slot_id.into();

        data["committee_end"] = committee_end;

        let mut proposals = json::JsonValue::new_array();

        for proposal in self.proposals().iter() {
            let mut item = json::JsonValue::new_object();
            item["external_id"] = proposal.external_id().to_string().into();
            item["options"] = proposal.options().choice_range().end.into();
            item["action"] = json::JsonValue::String("off_chain".to_string());
            let _ = proposals.push(item);
        }

        data["proposals"] = proposals;
        data["committee_member_public_keys"] = json::array![];
        data
    }

    fn as_json_str(&self) -> String {
        let data = self.as_json();
        json::stringify_pretty(data, 3)
    }
}
