use chain_core::property::FromStr;
use chain_impl_mockchain::{
    certificate::{VotePlan, VoteTallyPayload},
    vote::Choice,
};
use chain_time::TimeEra;
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::BlockDate as BlockDateDto;
use jormungandr_testing_utils::testing::MemPoolCheck;
use jormungandr_testing_utils::testing::SyncNode;
use jormungandr_testing_utils::testing::VoteCastCounter;
use jormungandr_testing_utils::testing::{RemoteJormungandr, VotePlanBuilder};
use jortestkit::load::{Request, RequestFailure, RequestGenerator};
use rand::RngCore;
use rand_core::OsRng;
use std::iter;
use std::time::Duration;
use std::time::Instant;
use thor::{
    FragmentBuilder, FragmentSender, FragmentSenderError, FragmentVerifier, StakePool, Wallet,
};

pub struct FragmentGenerator<'a, S: SyncNode + Send> {
    sender: Wallet,
    receiver: Wallet,
    active_stake_pools: Vec<StakePool>,
    vote_plans_for_casting: Vec<VotePlan>,
    vote_plans_for_tally: Vec<VotePlan>,
    node: RemoteJormungandr,
    rand: OsRng,
    vote_cast_register: Option<VoteCastCounter>,
    slots_per_epoch: u32,
    fragment_sender: FragmentSender<'a, S>,
    stake_pools_count: usize,
    vote_plans_for_tally_count: usize,
    vote_plans_for_casting_count: usize,
}

impl<'a, S: SyncNode + Send> FragmentGenerator<'a, S> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sender: Wallet,
        receiver: Wallet,
        node: RemoteJormungandr,
        slots_per_epoch: u32,
        stake_pools_count: usize,
        vote_plans_for_tally_count: usize,
        vote_plans_for_casting_count: usize,
        fragment_sender: FragmentSender<'a, S>,
    ) -> Self {
        assert!(vote_plans_for_casting_count > 1);
        assert!(stake_pools_count > 1);
        assert!(vote_plans_for_tally_count > 1);

        Self {
            sender,
            receiver,
            active_stake_pools: vec![],
            vote_plans_for_casting: vec![],
            vote_plans_for_tally: vec![],
            node,
            vote_cast_register: None,
            rand: OsRng,
            slots_per_epoch,
            fragment_sender,
            stake_pools_count,
            vote_plans_for_tally_count,
            vote_plans_for_casting_count,
        }
    }

    pub fn active_stake_pools(&self) -> Vec<StakePool> {
        self.active_stake_pools.clone()
    }

    pub fn prepare(&mut self, start_block_date: BlockDateDto) {
        let time_era = start_block_date.time_era(self.slots_per_epoch);
        let mut fragments = Vec::new();
        let settings = self.node.rest().settings().unwrap();
        let block0_hash = Hash::from_str(&settings.block0_hash).unwrap();
        let fees = settings.fees;

        let stake_pools: Vec<StakePool> = iter::from_fn(|| Some(StakePool::new(&self.sender)))
            .take(self.stake_pools_count)
            .collect();

        let votes_plan_for_casting: Vec<VotePlan> = iter::from_fn(|| {
            Some(
                VotePlanBuilder::new()
                    .proposals_count(256)
                    .vote_start(start_block_date.shift_slot(5, &time_era).into())
                    .tally_start(start_block_date.shift_epoch(5).into())
                    .tally_end(start_block_date.shift_epoch(6).into())
                    .build(),
            )
        })
        .take(self.vote_plans_for_casting_count)
        .collect();

        let vote_plans_for_tally: Vec<VotePlan> = iter::from_fn(|| {
            Some(
                VotePlanBuilder::new()
                    .vote_start(start_block_date.shift_slot(10, &time_era).into())
                    .tally_start(start_block_date.shift_slot(11, &time_era).into())
                    .tally_end(start_block_date.shift_epoch(5).into())
                    .build(),
            )
        })
        .take(self.vote_plans_for_tally_count)
        .collect();

        for stake_pool in &stake_pools {
            fragments.push(
                FragmentBuilder::new(&block0_hash, &fees, self.fragment_sender.date())
                    .stake_pool_registration(&self.sender, stake_pool),
            );
            self.sender.confirm_transaction();
        }
        for vote_plan_for_casting in &votes_plan_for_casting {
            fragments.push(
                FragmentBuilder::new(&block0_hash, &fees, self.fragment_sender.date())
                    .vote_plan(&self.sender, vote_plan_for_casting),
            );
            self.sender.confirm_transaction();
        }

        for vote_plan_for_tally in &vote_plans_for_tally {
            fragments.push(
                FragmentBuilder::new(&block0_hash, &fees, self.fragment_sender.date())
                    .vote_plan(&self.sender, vote_plan_for_tally),
            );
            self.sender.confirm_transaction();
        }

        self.fragment_sender
            .send_batch_fragments(fragments, true, &self.node)
            .unwrap();
        FragmentVerifier::wait_for_all_fragments(Duration::from_secs(10), &self.node).unwrap();
        self.vote_plans_for_casting = votes_plan_for_casting;
        self.vote_plans_for_tally = vote_plans_for_tally;
        self.active_stake_pools = stake_pools;
        self.vote_cast_register = Some(VoteCastCounter::new(
            1,
            self.vote_plans_for_casting
                .iter()
                .map(|x| (x.to_id(), x.proposals().len() as u8))
                .collect(),
        ));
    }

    pub fn send_random(&mut self) -> Result<MemPoolCheck, FragmentSenderError> {
        let rand = self.rand.next_u32() as u8;
        self.send_one(rand)
    }

    pub fn send_all(&mut self) -> Result<Vec<MemPoolCheck>, FragmentSenderError> {
        let mut checks = Vec::new();
        for i in 0..10 {
            checks.push(self.send_one(i as u8)?);
        }
        Ok(checks)
    }

    pub fn send_one(&mut self, option: u8) -> Result<MemPoolCheck, FragmentSenderError> {
        match option % 10 {
            0 => self.fragment_sender.send_transaction(
                &mut self.sender,
                &self.receiver,
                &self.node,
                1.into(),
            ),
            1 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.get(index).unwrap();

                self.fragment_sender
                    .send_full_delegation(&mut self.sender, stake_pool, &self.node)
            }
            2 => {
                let (left, right) = self.active_stake_pools.split_first().unwrap();

                self.fragment_sender.send_split_delegation(
                    &mut self.sender,
                    &[(left, 1), (right.first().unwrap(), 1)],
                    &self.node,
                )
            }
            3 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.get(index).unwrap();

                self.fragment_sender
                    .send_owner_delegation(&mut self.sender, stake_pool, &self.node)
            }
            4 => {
                let stake_pool = StakePool::new(&self.sender);
                self.active_stake_pools.push(stake_pool.clone());
                self.fragment_sender.send_pool_registration(
                    &mut self.sender,
                    &stake_pool,
                    &self.node,
                )
            }
            5 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.get(index).unwrap();
                self.fragment_sender.send_pool_update(
                    &mut self.sender,
                    stake_pool,
                    stake_pool,
                    &self.node,
                )
            }
            6 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.remove(index);

                self.fragment_sender
                    .send_pool_retire(&mut self.sender, &stake_pool, &self.node)
            }
            7 => {
                let block_date = BlockDateDto::from_str(
                    self.node
                        .rest()
                        .stats()
                        .unwrap()
                        .stats
                        .unwrap()
                        .last_block_date
                        .unwrap()
                        .as_ref(),
                )
                .unwrap();

                let time_era = TimeEra::new(
                    (block_date.slot() as u64).into(),
                    chain_time::Epoch(block_date.epoch()),
                    self.slots_per_epoch,
                );
                let vote_plan = VotePlanBuilder::new()
                    .vote_start(block_date.shift_slot(5, &time_era).into())
                    .tally_start(block_date.shift_slot(6, &time_era).into())
                    .tally_end(block_date.shift_epoch(4).into())
                    .build();
                self.fragment_sender
                    .send_vote_plan(&mut self.sender, &vote_plan, &self.node)
            }
            8 => {
                let vote_cast_register = self
                    .vote_cast_register
                    .as_mut()
                    .expect("please run 'prepare' method before running load");

                // wallet_idx is always 0 because we are using only one wallet
                let wallet_idx = 0;
                let wallet_votes_to_cast = vote_cast_register.advance_single(wallet_idx).unwrap();
                let votes_to_cast = wallet_votes_to_cast.get(0).unwrap();
                let vote_plan = self
                    .vote_plans_for_casting
                    .iter()
                    .find(|x| x.to_id() == votes_to_cast.id())
                    .unwrap();

                self.fragment_sender.send_vote_cast(
                    &mut self.sender,
                    vote_plan,
                    votes_to_cast.range().start as u8,
                    &Choice::new(1),
                    &self.node,
                )
            }
            9 => {
                let index = self.rand.next_u32() as usize % self.vote_plans_for_tally.len();
                let vote_plan = self.vote_plans_for_tally.get(index).unwrap();

                self.fragment_sender.send_vote_tally(
                    &mut self.sender,
                    vote_plan,
                    &self.node,
                    VoteTallyPayload::Public,
                )
            }
            _ => unreachable!(),
        }
    }
}

impl<'a, S: SyncNode + Send + Sync> RequestGenerator for FragmentGenerator<'a, S> {
    fn next(&mut self) -> Result<Request, RequestFailure> {
        let start = Instant::now();
        self.send_random()
            .map(|x| Request {
                ids: vec![Some(x.fragment_id().to_string())],
                duration: start.elapsed(),
            })
            .map_err(|err| RequestFailure::General(err.to_string()))
    }

    fn split(self) -> (Self, Option<Self>) {
        (self, None)
    }
}
