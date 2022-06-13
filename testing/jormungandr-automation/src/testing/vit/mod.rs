mod builder;
mod vote_cast_register;

pub use builder::VotePlanBuilder;
use chain_crypto::bech32::Bech32;
use chain_impl_mockchain::{certificate::VotePlan, vote::PayloadType};
pub use vote_cast_register::VoteCastCounter;

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

            match proposal.action() {
                chain_impl_mockchain::certificate::VoteAction::OffChain => {
                    item["action"] = json::JsonValue::String("off_chain".to_string());
                }
                chain_impl_mockchain::certificate::VoteAction::Treasury { action } => {
                    match action {
                        chain_impl_mockchain::ledger::governance::TreasuryGovernanceAction::NoOp => {
                            unimplemented!()
                        }
                        chain_impl_mockchain::ledger::governance::TreasuryGovernanceAction::TransferToRewards { value } => {
                            item["action"] = json::parse(&format!(r#"
                                                {{
                                                    "treasury": {{
                                                        "transfer_to_rewards": {{
                                                            "value": {}
                                                        }}
                                                    }}
                                                }}"#,value)).unwrap();
                        }
                    }
                }
                chain_impl_mockchain::certificate::VoteAction::Parameters { action } => {
                    match action {
                        chain_impl_mockchain::ledger::governance::ParametersGovernanceAction::NoOp => {
                            unimplemented!()
                        }
                        chain_impl_mockchain::ledger::governance::ParametersGovernanceAction::RewardAdd { value } => {
                            item["action"] = json::parse(&format!(r#"
                            {{
                                "governance": {{
                                    "reward_add": {{
                                        "value": {}
                                    }},
                                }}
                            }}"#,value)).unwrap();
                        }
                    }
                }
            }
            let _ = proposals.push(item);
        }

        data["proposals"] = proposals;

        let mut committee_member_public_keys = json::JsonValue::new_array();

        for member in self.committee_public_keys() {
            let _ = committee_member_public_keys.push(member.to_bech32_str());
        }

        data["committee_member_public_keys"] = committee_member_public_keys;
        data["voting_token"] = self.voting_token().to_string().into();
        data
    }

    fn as_json_str(&self) -> String {
        let data = self.as_json();
        json::stringify_pretty(data, 3)
    }
}
