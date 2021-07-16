use crate::testing::SyncNode;
use crate::{
    testing::{FragmentSender, FragmentSenderError, MemPoolCheck, RemoteJormungandr},
    wallet::Wallet,
};
use chain_impl_mockchain::{certificate::VotePlan, vote::Choice};
use jortestkit::load::{Request, RequestFailure, RequestGenerator};
use rand::RngCore;
use rand_core::OsRng;
use std::time::Instant;

pub struct VoteCastsGenerator<'a, S: SyncNode + Send> {
    voters: Vec<Wallet>,
    vote_plan: VotePlan,
    node: RemoteJormungandr,
    rand: OsRng,
    fragment_sender: FragmentSender<'a, S>,
    send_marker: usize,
}

impl<'a, S: SyncNode + Send> VoteCastsGenerator<'a, S> {
    pub fn new(
        voters: Vec<Wallet>,
        vote_plan: VotePlan,
        node: RemoteJormungandr,
        fragment_sender: FragmentSender<'a, S>,
    ) -> Self {
        Self {
            voters,
            vote_plan,
            node,
            rand: OsRng,
            fragment_sender,
            send_marker: 0,
        }
    }

    pub fn send(&mut self) -> Result<MemPoolCheck, FragmentSenderError> {
        self.send_marker = (self.send_marker + 1) % self.voters.len();

        let mut voter = self.voters.get_mut(self.send_marker).unwrap();

        let choice = Choice::new((self.rand.next_u32() % 3) as u8);
        let proposal_index = self.rand.next_u32() % (self.vote_plan.proposals().len() as u32);

        self.fragment_sender.send_vote_cast(
            &mut voter,
            &self.vote_plan,
            proposal_index as u8,
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
        };
        (self, Some(other))
    }
}
