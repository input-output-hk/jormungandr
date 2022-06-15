use chain_impl_mockchain::{certificate::VotePlan, vote::Choice};
use jormungandr_automation::{
    jormungandr::{MemPoolCheck, RemoteJormungandr},
    testing::{SyncNode, VoteCastCounter},
};
use jortestkit::load::{Request, RequestFailure, RequestGenerator};
use rand_core::{OsRng, RngCore};
use std::time::Instant;
use thor::{FragmentSender, FragmentSenderError, Wallet};

pub struct VoteCastsGenerator<'a, S: SyncNode + Send> {
    voters: Vec<Wallet>,
    vote_plan: VotePlan,
    node: RemoteJormungandr,
    rand: OsRng,
    fragment_sender: FragmentSender<'a, S>,
    send_marker: usize,
    votes_register: VoteCastCounter,
}

impl<'a, S: SyncNode + Send> VoteCastsGenerator<'a, S> {
    pub fn new(
        voters: Vec<Wallet>,
        vote_plan: VotePlan,
        node: RemoteJormungandr,
        fragment_sender: FragmentSender<'a, S>,
    ) -> Self {
        let votes_register = VoteCastCounter::from_vote_plan(voters.len(), &vote_plan);

        Self {
            voters,
            vote_plan,
            node,
            rand: OsRng,
            fragment_sender,
            send_marker: 0,
            votes_register,
        }
    }

    pub fn send(&mut self) -> Result<MemPoolCheck, FragmentSenderError> {
        self.send_marker = (self.send_marker + 1) % self.voters.len();

        let voter = self.voters.get_mut(self.send_marker).unwrap();
        let vote_casts = self
            .votes_register
            .advance_single(self.send_marker)
            .unwrap();
        let vote_cast = vote_casts.get(0).unwrap();

        let choice = Choice::new((self.rand.next_u32() % 3) as u8);

        self.fragment_sender.send_vote_cast(
            voter,
            &self.vote_plan,
            vote_cast.first(),
            &choice,
            &self.node,
        )
    }
}

impl<'a, S: SyncNode + Send + Sync + Clone> RequestGenerator for VoteCastsGenerator<'a, S> {
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
        let wallets_len = self.voters.len();
        if wallets_len <= 1 {
            return (self, None);
        }
        let voters = self.voters.split_off(wallets_len / 2);
        let other = Self {
            voters,
            vote_plan: self.vote_plan.clone(),
            node: self.node.clone_with_rest(),
            fragment_sender: self.fragment_sender.clone(),
            rand: OsRng,
            send_marker: 1,
            votes_register: self.votes_register.clone(),
        };
        (self, Some(other))
    }
}
