pub struct VotePlanBuilder {
    proposals_count: u32,
    action: VoteAction,
    payload: PayloadType,
    member_keys: Vec<MemberPublicKey>,
}

impl VotePlanBuilder {
    pub fn new() -> Self {
        Self {
            proposals_count: 3,
            action: VoteAction::OffChain,
            payload: PayloadType::Public,
            member_keys: vec![],
        }
    }

    pub fn proposals_count(&mut self, proposals_count: u32) -> &mut Self {
        self.proposals_count = proposals_count;
        self
    }

    pub fn action_type(&mut self, action: VoteAction) -> &mut Self {
        self.action = action;
        self
    }

    pub fn private(&mut self) -> &mut Self {
        self.payload = PayloadType::Private;
        self
    }

    pub fn public(&mut self) -> &mut Self {
        self.payload = PayloadType::Public;
        self
    }

    pub fn member_public_key(&mut self, key: MemberPublicKey) -> &mut Self {
        self.member_keys.push(key);
        self
    }

    pub fn member_public_keys(&mut self, keys: &[MemberPublicKey]) -> &mut Self {
        self.member_keys.extend(keys.iter());
        self
    }

    pub fn build(&self) -> VotePlan {
        let proposals: Vec<Proposal> = std::iter::from_fn(|| {
            Some(Proposal::new(
                VoteTestGen::external_proposal_id(),
                Options::new_length(3).unwrap(),
                self.action_type.clone(),
            ))
        })
        .take(self.proposals_count)
        .collect();

        VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 0),
            BlockDate::from_epoch_slot_id(2, 0),
            BlockDate::from_epoch_slot_id(3, 0),
            proposals,
            self.payload_type,
            vec![],
        )
    }
}
/*
    let action = VoteAction::Parameters {
        action: ParametersGovernanceAction::RewardAdd {
            value: Value(rewards_increase),
        },
    };

*/
