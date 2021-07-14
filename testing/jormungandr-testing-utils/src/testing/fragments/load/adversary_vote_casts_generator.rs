use crate::testing::FragmentBuilder;
use crate::testing::SyncNode;
use crate::{
    testing::{FragmentSender, FragmentSenderError, MemPoolCheck, RemoteJormungandr},
    wallet::Wallet,
};
use chain_impl_mockchain::fragment::Fragment;
use chain_impl_mockchain::testing::VoteTestGen;
use chain_impl_mockchain::vote::PayloadType;
use chain_impl_mockchain::{certificate::VotePlan, vote::Choice};
use jortestkit::load::{Request, RequestFailure, RequestGenerator};
use rand::RngCore;
use rand_core::OsRng;
use std::time::Instant;

pub struct AdversaryVoteCastsGenerator<'a, S: SyncNode + Send> {
    voter: Wallet,
    vote_plans: Vec<VotePlan>,
    voting_privacy: PayloadType,
    node: RemoteJormungandr,
    rand: OsRng,
    fragment_sender: FragmentSender<'a, S>,
}

impl<'a, S: SyncNode + Send> AdversaryVoteCastsGenerator<'a, S> {
    #[allow(dead_code)]
    pub fn new(
        voter: Wallet,
        vote_plans: Vec<VotePlan>,
        node: RemoteJormungandr,
        fragment_sender: FragmentSender<'a, S>,
    ) -> Self {
        let voting_privacy = vote_plans.get(0).unwrap().payload_type();

        Self {
            voter,
            vote_plans,
            voting_privacy,
            node,
            rand: OsRng,
            fragment_sender,
        }
    }

    pub fn send(&mut self) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = match self.rand.next_u32() % 4 {
            0 => self.wrong_vote_plan(),
            1 => self.wrong_proposal_index(),
            2 => self.wrong_voting_privacy(),
            3 => self.wrong_choice(),
            _ => unimplemented!(),
        };
        self.fragment_sender
            .send_fragment(&mut self.voter, fragment, &self.node)
    }

    fn wrong_vote_plan(&self) -> Fragment {
        let vote_plan = VoteTestGen::vote_plan();
        let block0_hash = self.fragment_sender.block0_hash();
        let fees = self.fragment_sender.fees();

        match self.voting_privacy {
            PayloadType::Public => FragmentBuilder::new(&block0_hash, &fees).public_vote_cast(
                &self.voter,
                &vote_plan,
                0,
                &Choice::new(0),
            ),
            PayloadType::Private => FragmentBuilder::new(&block0_hash, &fees).private_vote_cast(
                &self.voter,
                &vote_plan,
                0,
                &Choice::new(0),
            ),
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
            PayloadType::Public => FragmentBuilder::new(&block0_hash, &fees).public_vote_cast(
                &self.voter,
                vote_plan,
                255,
                &Choice::new(0),
            ),
            PayloadType::Private => FragmentBuilder::new(&block0_hash, &fees).private_vote_cast(
                &self.voter,
                vote_plan,
                255,
                &Choice::new(0),
            ),
        }
    }

    fn wrong_voting_privacy(&self) -> Fragment {
        let vote_plan = self.vote_plans.get(0).unwrap();
        let block0_hash = self.fragment_sender.block0_hash();
        let fees = self.fragment_sender.fees();

        match self.voting_privacy {
            PayloadType::Public => FragmentBuilder::new(&block0_hash, &fees).private_vote_cast(
                &self.voter,
                vote_plan,
                0,
                &Choice::new(0),
            ),
            PayloadType::Private => FragmentBuilder::new(&block0_hash, &fees).public_vote_cast(
                &self.voter,
                vote_plan,
                0,
                &Choice::new(0),
            ),
        }
    }

    fn wrong_choice(&self) -> Fragment {
        let vote_plan = self.vote_plans.get(0).unwrap();
        let options: u8 = vote_plan.proposals()[0].options().choice_range().end + 1;
        let block0_hash = self.fragment_sender.block0_hash();
        let fees = self.fragment_sender.fees();

        match self.voting_privacy {
            PayloadType::Public => FragmentBuilder::new(&block0_hash, &fees).public_vote_cast(
                &self.voter,
                vote_plan,
                0,
                &Choice::new(options),
            ),
            PayloadType::Private => FragmentBuilder::new(&block0_hash, &fees).private_vote_cast(
                &self.voter,
                vote_plan,
                0,
                &Choice::new(options),
            ),
        }
    }
}

impl<'a, S: SyncNode + Send + Sync> RequestGenerator for AdversaryVoteCastsGenerator<'a, S> {
    fn next(&mut self) -> Result<Request, RequestFailure> {
        let start = Instant::now();
        self.send()
            .map(|x| Request {
                ids: vec![Some(x.fragment_id().to_string())],
                duration: start.elapsed(),
            })
            .map_err(|err| RequestFailure::General(err.to_string()))
    }

    fn split(self) -> (Self, Option<Self>) {
        // TODO: implement real splitting
        (self, None)
    }
}
