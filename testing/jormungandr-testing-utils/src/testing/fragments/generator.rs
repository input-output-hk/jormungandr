use super::{FragmentSender, FragmentSenderError, MemPoolCheck};
use crate::testing::node::Explorer;
use crate::wallet::Wallet;
use crate::{
    stake_pool::StakePool,
    testing::{FragmentNode, SyncNode},
};
use chain_impl_mockchain::{certificate::VotePlan, vote::Choice};
use rand::RngCore;
use rand_core::OsRng;

pub struct FragmentGenerator<'a, Node> {
    sender: &'a Wallet,
    receiver: &'a Wallet,
    active_stake_pools: Vec<StakePool>,
    vote_plan_for_casting: VotePlan,
    vote_plans_for_tally: Vec<VotePlan>,
    node: &'a Node,
    rand: OsRng,
    explorer: Explorer,
    fragment_sender: &'a FragmentSender<'a>,
}

impl<'a, Node: FragmentNode + SyncNode + Sized + Sync + Send> FragmentGenerator<'a, Node> {
    pub fn new(
        sender: &'a Wallet,
        receiver: &'a Wallet,
        stake_pools: Vec<StakePool>,
        vote_plan_for_casting: VotePlan,
        vote_plans_for_tally: Vec<VotePlan>,
        node: &'a Node,
        explorer: Explorer,
        fragment_sender: &'a FragmentSender,
    ) -> Self {
        Self {
            sender,
            receiver,
            active_stake_pools: stake_pools,
            vote_plan_for_casting,
            vote_plans_for_tally,
            node,
            rand: OsRng,
            explorer,
            fragment_sender,
        }
    }

    pub fn send_random(&mut self) -> Result<MemPoolCheck, FragmentSenderError> {
        self.send_one(self.rand.next_u32() as u8)
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
                self.receiver,
                self.node,
                1.into(),
            ),
            1 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.get(index).unwrap();

                self.fragment_sender
                    .send_full_delegation(&mut self.sender, stake_pool, self.node)
            }
            2 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.get(index).unwrap();

                self.fragment_sender.send_split_delegation(
                    &mut self.sender,
                    &vec![(stake_pool, 1)],
                    self.node,
                )
            }
            3 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.get(index).unwrap();

                self.fragment_sender
                    .send_owner_delegation(&mut self.sender, stake_pool, self.node)
            }
            4 => {
                let stake_pool = StakePool::new(self.sender);
                self.active_stake_pools.push(stake_pool.clone());
                self.fragment_sender.send_pool_registration(
                    &mut self.sender,
                    &stake_pool,
                    self.node,
                )
            }
            5 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.get(index).unwrap();
                self.fragment_sender.send_pool_update(
                    &mut self.sender,
                    stake_pool,
                    stake_pool,
                    self.node,
                )
            }
            6 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.remove(index);

                self.fragment_sender
                    .send_pool_retire(&mut self.sender, &stake_pool, self.node)
            }
            7 => {
                let block_date = self.explorer.current_time();

                let vote_plan = VotePlan::new_with_dates(
                    block_date.next(5),
                    block_date.next(),
                    block_date.next_epoch(4),
                );
                self.vote_plans_for_tally.push(vote_plan);
                self.fragment_sender
                    .send_vote_plan(&mut self.sender, &vote_plan, self.node)
            }
            8 => self.fragment_sender.send_vote_cast(
                &mut self.sender,
                &self.vote_plan_for_casting,
                0,
                &Choice::new(1),
                self.node,
            ),
            9 => {
                let index = self.rand.next_u32() as usize % self.vote_plans_for_tally.len();
                let vote_plan = self.vote_plans_for_tally.remove(index);

                self.fragment_sender
                    .send_vote_tally(&mut self.sender, &vote_plan, self.node)
            }
            10 => unreachable!(),
        }
    }
}
