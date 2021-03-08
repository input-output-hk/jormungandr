use super::{FragmentSender, FragmentSenderError, MemPoolCheck};
use crate::testing::SyncNode;
use crate::{
    stake_pool::StakePool,
    testing::{node::Explorer, RemoteJormungandr, VotePlanBuilder},
    wallet::Wallet,
};
use chain_impl_mockchain::{
    certificate::{VotePlan, VoteTallyPayload},
    vote::Choice,
};
use chain_time::TimeEra;
use jormungandr_lib::interfaces::BlockDate;
use jortestkit::load::{RequestFailure, RequestGenerator};
use rand::RngCore;
use rand_core::OsRng;
use std::iter;

pub struct FragmentGenerator<'a, S: SyncNode + Send> {
    sender: Wallet,
    receiver: Wallet,
    active_stake_pools: Vec<StakePool>,
    vote_plan_for_casting: Option<VotePlan>,
    vote_plans_for_tally: Vec<VotePlan>,
    node: RemoteJormungandr,
    rand: OsRng,
    explorer: Explorer,
    slots_per_epoch: u32,
    fragment_sender: FragmentSender<'a, S>,
}

impl<'a, S: SyncNode + Send> FragmentGenerator<'a, S> {
    pub fn new(
        sender: Wallet,
        receiver: Wallet,
        node: RemoteJormungandr,
        explorer: Explorer,
        slots_per_epoch: u32,
        fragment_sender: FragmentSender<'a, S>,
    ) -> Self {
        Self {
            sender,
            receiver,
            active_stake_pools: vec![],
            vote_plan_for_casting: None,
            vote_plans_for_tally: vec![],
            node,
            rand: OsRng,
            explorer,
            slots_per_epoch,
            fragment_sender,
        }
    }

    pub fn prepare(&mut self, start_block_date: BlockDate) {
        let time_era = start_block_date.time_era(self.slots_per_epoch);

        let stake_pools: Vec<StakePool> = iter::from_fn(|| Some(StakePool::new(&self.sender)))
            .take(30)
            .collect();

        for stake_pool in stake_pools.iter() {
            self.fragment_sender
                .send_pool_registration(&mut self.sender, &stake_pool, &self.node)
                .unwrap();
        }

        let vote_plan_for_casting: VotePlan = VotePlanBuilder::new()
            .with_vote_start(start_block_date.into())
            .with_tally_start(start_block_date.shift_epoch(5).into())
            .with_tally_end(start_block_date.shift_epoch(6).into())
            .build();

        self.fragment_sender
            .send_vote_plan(&mut self.sender, &vote_plan_for_casting, &self.node)
            .unwrap();
        let vote_plans_for_tally: Vec<VotePlan> = iter::from_fn(|| {
            Some(
                VotePlanBuilder::new()
                    .with_vote_start(start_block_date.into())
                    .with_tally_start(start_block_date.shift_slot(1, &time_era).into())
                    .with_tally_end(start_block_date.shift_epoch(5).into())
                    .build(),
            )
        })
        .take(30)
        .collect();

        for vote_plan in vote_plans_for_tally.iter() {
            println!("{:?}", vote_plan);

            self.fragment_sender
                .send_vote_plan(&mut self.sender, &vote_plan, &self.node)
                .unwrap();
        }
        self.vote_plan_for_casting = Some(vote_plan_for_casting);
        self.vote_plans_for_tally = vote_plans_for_tally;
        self.active_stake_pools = stake_pools;
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
                let block_date = self.explorer.current_time();

                let time_era = TimeEra::new(
                    (block_date.slot() as u64).into(),
                    chain_time::Epoch(block_date.epoch()),
                    self.slots_per_epoch,
                );
                let vote_plan = VotePlanBuilder::new()
                    .with_vote_start(block_date.shift_slot(5, &time_era).into())
                    .with_tally_start(block_date.shift_slot(6, &time_era).into())
                    .with_tally_end(block_date.shift_epoch(4).into())
                    .build();
                self.fragment_sender
                    .send_vote_plan(&mut self.sender, &vote_plan, &self.node)
            }
            8 => self.fragment_sender.send_vote_cast(
                &mut self.sender,
                self.vote_plan_for_casting.as_ref().unwrap(),
                0,
                &Choice::new(1),
                &self.node,
            ),
            9 => {
                let index = self.rand.next_u32() as usize % self.vote_plans_for_tally.len();
                let vote_plan = self.vote_plans_for_tally.remove(index);

                self.fragment_sender.send_vote_tally(
                    &mut self.sender,
                    &vote_plan,
                    &self.node,
                    VoteTallyPayload::Public,
                )
            }
            _ => unreachable!(),
        }
    }
}

impl<'a, S: SyncNode + Send> RequestGenerator for FragmentGenerator<'a, S> {
    fn next(
        &mut self,
    ) -> Result<Vec<Option<jortestkit::load::Id>>, jortestkit::load::RequestFailure> {
        self.send_random()
            .map(|x| vec![Some(x.fragment_id().to_string())])
            .map_err(|err| RequestFailure::General(err.to_string()))
    }
}
