use chain_impl_mockchain::vote::PayloadType;
mod builder;
use bech32::ToBase32;
pub use builder::VotePlanBuilder;
use chain_impl_mockchain::certificate::VotePlan;

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

        let mut committee_member_public_keys = json::JsonValue::new_array();

        for member in self.committee_public_keys() {
            let encoded_member_key = bech32::encode(
                jormungandr_lib::interfaces::MEMBER_PUBLIC_KEY_BECH32_HRP,
                member.to_bytes().to_base32(),
            )
            .unwrap();
            let _ = committee_member_public_keys.push(encoded_member_key);
        }

        data["committee_member_public_keys"] = committee_member_public_keys;
        data
    }

    fn as_json_str(&self) -> String {
        let data = self.as_json();
        json::stringify_pretty(data, 3)
    }
}
