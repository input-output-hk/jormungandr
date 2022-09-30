use crate::{
    builder::{VotePlanSettings, Wallet},
    config::VotePlanTemplate,
};
use chain_impl_mockchain::{
    fragment::Fragment, testing::create_initial_vote_plan, vote::PayloadType,
};
use chain_vote::Crs;
use jormungandr_lib::interfaces::VotePlan;
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thor::CommitteeDataManager;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct VotePlanKey {
    pub alias: String,
    pub owner_alias: String,
}

pub fn generate_vote_plans(
    wallets: &[Wallet],
    vote_plans: &[VotePlanTemplate],
    keys: &mut CommitteeDataManager,
) -> (HashMap<VotePlanKey, VotePlanSettings>, Vec<Fragment>) {
    let mut vote_plans_fragments = Vec::new();
    let mut vote_plans_settings = HashMap::new();

    for template in vote_plans.iter() {
        let owner = wallets
            .iter().find(|wallet| wallet.template().has_alias(&template.vote_plan_key.owner_alias))
            .unwrap_or_else(|| {
                panic!(
                    "Owner {} of {} is an unknown or not generated wallet (we need to have its private key for signatures) ",
                    template.vote_plan_key.owner_alias, template.vote_plan_key.alias
                )
            });

        let mut vote_plan = VotePlan {
            payload_type: {
                if template.private.is_some() {
                    PayloadType::Private.into()
                } else {
                    PayloadType::Public.into()
                }
            },
            vote_start: template.vote_start,
            vote_end: template.vote_end,
            committee_end: template.committee_end,
            proposals: template.proposals.clone(),
            committee_member_public_keys: vec![],
            voting_token: template.voting_token.clone(),
        };

        let vote_plan_settings: VotePlanSettings =
            if let Some(private_parameters) = &template.private {
                if keys.membership.committees().is_empty() {
                    if let Some(communication) = &keys.communication {
                        let mut rng = rand::thread_rng();

                        let crs: Crs = {
                            if let Some(crs) = &private_parameters.crs {
                                Crs::from_hash(crs.as_bytes())
                            } else {
                                let mut buf = [0; 32];
                                rng.fill_bytes(&mut buf);
                                Crs::from_hash(&buf)
                            }
                        };
                        keys.membership = communication.membership_data(
                            crs,
                            private_parameters.threshold.unwrap_or(1),
                            &mut rng,
                        );
                    }
                }

                vote_plan.committee_member_public_keys = keys.member_public_keys();
                VotePlanSettings::Private {
                    keys: keys.clone(),
                    vote_plan: vote_plan.clone(),
                }
            } else {
                VotePlanSettings::from_public_vote_plan(vote_plan.clone())
            };

        vote_plans_fragments.push(create_initial_vote_plan(
            &vote_plan_settings.vote_plan().into(),
            &[owner.clone().try_into().unwrap()],
        ));

        vote_plans_settings.insert(template.vote_plan_key.clone(), vote_plan_settings);
    }

    (vote_plans_settings, vote_plans_fragments)
}
