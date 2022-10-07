use crate::builder::VotePlanKey;
use chain_impl_mockchain::{
    certificate::{VoteAction, VotePlan},
    ledger::governance::{ParametersGovernanceAction, TreasuryGovernanceAction},
    testing::scenario::template::{ProposalDefBuilder, VotePlanDef, VotePlanDefBuilder},
};

pub trait VotePlanExtension {
    fn convert_to_def(self, key: &VotePlanKey) -> VotePlanDef;
}

impl VotePlanExtension for VotePlan {
    fn convert_to_def(self, key: &VotePlanKey) -> VotePlanDef {
        let mut builder = VotePlanDefBuilder::new(&key.alias);
        builder
            .owner(&key.owner_alias)
            .payload_type(self.payload_type())
            .committee_keys(self.committee_public_keys().to_vec())
            .voting_token(self.voting_token().clone())
            .vote_phases(
                self.vote_start().epoch,
                self.committee_start().epoch,
                self.committee_end().epoch,
            );

        for proposal in self.proposals().iter() {
            let mut proposal_builder = ProposalDefBuilder::new(proposal.external_id().clone());

            let length = proposal
                .options()
                .choice_range()
                .end
                .checked_sub(proposal.options().choice_range().start)
                .unwrap();

            proposal_builder.options(length);

            match proposal.action() {
                VoteAction::OffChain => {
                    proposal_builder.action_off_chain();
                }
                VoteAction::Treasury { action } => match action {
                    TreasuryGovernanceAction::TransferToRewards { value } => {
                        proposal_builder.action_rewards_add(value.0);
                    }
                    TreasuryGovernanceAction::NoOp => {
                        unimplemented!();
                    }
                },
                VoteAction::Parameters { action } => match action {
                    ParametersGovernanceAction::RewardAdd { value } => {
                        proposal_builder.action_transfer_to_rewards(value.0);
                    }
                    ParametersGovernanceAction::NoOp => {
                        proposal_builder.action_parameters_no_op();
                    }
                },
            };

            builder.with_proposal(&mut proposal_builder);
        }
        builder.build()
    }
}
