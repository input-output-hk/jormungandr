use chain_impl_mockchain::{
    certificate::VotePlan,
    fragment::Fragment,
    testing::VoteTestGen,
    vote::{Choice, PayloadType},
};
use jormungandr_automation::{
    jormungandr::{MemPoolCheck, RemoteJormungandr},
    testing::SyncNode,
};
use jortestkit::load::{Request, RequestFailure, RequestGenerator};
use rand::RngCore;
use rand_core::OsRng;
use std::time::Instant;
use thor::{BlockDateGenerator, FragmentBuilder, FragmentSender, FragmentSenderError, Wallet};

const DEFAULT_MAX_SPLITS: usize = 7; // equals to 128 splits, will likely not reach that value but it's there just to prevent a stack overflow

pub struct AdversaryVoteCastsGenerator<'a, S: SyncNode + Send> {
    expiry_generator: BlockDateGenerator,
    voter: Wallet,
    vote_plans: Vec<VotePlan>,
    voting_privacy: PayloadType,
    node: RemoteJormungandr,
    rand: OsRng,
    fragment_sender: FragmentSender<'a, S>,
    max_splits: usize,
}

impl<'a, S: SyncNode + Send> AdversaryVoteCastsGenerator<'a, S> {
    #[allow(dead_code)]
    pub fn new(
        expiry_generator: BlockDateGenerator,
        voter: Wallet,
        vote_plans: Vec<VotePlan>,
        node: RemoteJormungandr,
        fragment_sender: FragmentSender<'a, S>,
    ) -> Self {
        let voting_privacy = vote_plans.get(0).unwrap().payload_type();

        Self {
            expiry_generator,
            voter,
            vote_plans,
            voting_privacy,
            node,
            rand: OsRng,
            fragment_sender,
            max_splits: DEFAULT_MAX_SPLITS,
        }
    }

    fn send(&mut self) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = match self.rand.next_u32() % 4 {
            0 => self.wrong_vote_plan(),
            1 => self.wrong_proposal_index(),
            2 => self.wrong_voting_privacy(),
            3 => self.wrong_choice(),
            _ => unreachable!(),
        };
        self.fragment_sender
            .send_fragment(&mut self.voter, fragment, &self.node)
    }

    fn wrong_vote_plan(&self) -> Fragment {
        let vote_plan = VoteTestGen::vote_plan();
        let block0_hash = self.fragment_sender.block0_hash();
        let fees = self.fragment_sender.fees();

        match self.voting_privacy {
            PayloadType::Public => {
                FragmentBuilder::new(&block0_hash, &fees, self.expiry_generator.block_date())
                    .public_vote_cast(&self.voter, &vote_plan, 0, &Choice::new(0))
            }
            PayloadType::Private => {
                FragmentBuilder::new(&block0_hash, &fees, self.expiry_generator.block_date())
                    .private_vote_cast(&self.voter, &vote_plan, 0, &Choice::new(0))
            }
        }
    }

    fn wrong_proposal_index(&self) -> Fragment {
        let vote_plan = self
            .vote_plans
            .iter()
            .find(|x| x.proposals().len() < 256)
            .expect("cannot find vote plan with less than 256 proposals");
        let block0_hash = self.fragment_sender.block0_hash();
        let fees = self.fragment_sender.fees();

        match self.voting_privacy {
            PayloadType::Public => {
                FragmentBuilder::new(&block0_hash, &fees, self.expiry_generator.block_date())
                    .public_vote_cast(&self.voter, vote_plan, 255, &Choice::new(0))
            }
            PayloadType::Private => {
                FragmentBuilder::new(&block0_hash, &fees, self.expiry_generator.block_date())
                    .private_vote_cast(&self.voter, vote_plan, 255, &Choice::new(0))
            }
        }
    }

    fn wrong_voting_privacy(&self) -> Fragment {
        let vote_plan = self.vote_plans.get(0).unwrap();
        let block0_hash = self.fragment_sender.block0_hash();
        let fees = self.fragment_sender.fees();

        match self.voting_privacy {
            PayloadType::Public => {
                FragmentBuilder::new(&block0_hash, &fees, self.expiry_generator.block_date())
                    .private_vote_cast(&self.voter, vote_plan, 0, &Choice::new(0))
            }
            PayloadType::Private => {
                FragmentBuilder::new(&block0_hash, &fees, self.expiry_generator.block_date())
                    .public_vote_cast(&self.voter, vote_plan, 0, &Choice::new(0))
            }
        }
    }

    fn wrong_choice(&self) -> Fragment {
        let vote_plan = self.vote_plans.get(0).unwrap();
        let options: u8 = vote_plan.proposals()[0].options().choice_range().end + 1;
        let block0_hash = self.fragment_sender.block0_hash();
        let fees = self.fragment_sender.fees();

        match self.voting_privacy {
            PayloadType::Public => {
                FragmentBuilder::new(&block0_hash, &fees, self.expiry_generator.block_date())
                    .public_vote_cast(&self.voter, vote_plan, 0, &Choice::new(options))
            }
            PayloadType::Private => {
                FragmentBuilder::new(&block0_hash, &fees, self.expiry_generator.block_date())
                    .private_vote_cast(&self.voter, vote_plan, 0, &Choice::new(options))
            }
        }
    }
}

impl<'a, S: SyncNode + Send + Sync + Clone> RequestGenerator
    for AdversaryVoteCastsGenerator<'a, S>
{
    fn next(&mut self) -> Result<Request, RequestFailure> {
        let start = Instant::now();
        self.send()
            .map(|x| Request {
                ids: vec![Some(x.fragment_id().to_string())],
                duration: start.elapsed(),
            })
            .map_err(|err| RequestFailure::General(err.to_string()))
    }

    fn split(mut self) -> (Self, Option<Self>) {
        // Since no transaction will ever be accepted we could split as many times as we want
        // but that may trigger a bug in rayon so we artificially limit it
        if self.max_splits == 0 {
            return (self, None);
        }

        self.max_splits -= 1;

        let other = Self {
            expiry_generator: self.expiry_generator.clone(),
            voter: self.voter.clone(),
            vote_plans: self.vote_plans.clone(),
            voting_privacy: self.voting_privacy,
            node: self.node.clone_with_rest(),
            rand: OsRng,
            fragment_sender: self.fragment_sender.clone(),
            max_splits: self.max_splits,
        };
        (self, Some(other))
    }
}
