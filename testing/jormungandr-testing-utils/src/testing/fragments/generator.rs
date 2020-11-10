use super::{FragmentSender, FragmentSenderError, MemPoolCheck};
use crate::{
    stake_pool::StakePool,
    testing::{node::Explorer, FragmentNode, SyncNode, VotePlanBuilder},
    wallet::Wallet,
};
use chain_impl_mockchain::{certificate::VotePlan, vote::Choice};
use chain_time::TimeEra;
use rand::RngCore;
use rand_core::OsRng;

pub struct FragmentGenerator<'a, Node> {
    sender: &'a mut Wallet,
    receiver: &'a Wallet,
    active_stake_pools: Vec<StakePool>,
    vote_plan_for_casting: VotePlan,
    //    vote_plans_for_tally: Vec<VotePlan>,
    node: &'a Node,
    rand: OsRng,
    explorer: Explorer,
    slots_per_epoch: u32,
}

impl<'a, Node: FragmentNode + SyncNode + Sized + Sync + Send> FragmentGenerator<'a, Node> {
    pub fn new(
        sender: &'a mut Wallet,
        receiver: &'a Wallet,
        stake_pools: Vec<StakePool>,
        vote_plan_for_casting: VotePlan,
        //  vote_plans_for_tally: Vec<VotePlan>,
        node: &'a Node,
        explorer: Explorer,
        slots_per_epoch: u32,
    ) -> Self {
        Self {
            sender,
            receiver,
            active_stake_pools: stake_pools,
            vote_plan_for_casting,
            //    vote_plans_for_tally,
            node,
            rand: OsRng,
            explorer,
            slots_per_epoch,
        }
    }

    pub fn send_random(
        &mut self,
        fragment_sender: &'a FragmentSender,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let rand = self.rand.next_u32() as u8;
        self.send_one(rand, &fragment_sender)
    }

    pub fn send_all(
        &mut self,
        fragment_sender: &'a FragmentSender,
    ) -> Result<Vec<MemPoolCheck>, FragmentSenderError> {
        let mut checks = Vec::new();
        for i in 0..10 {
            checks.push(self.send_one(i as u8, &fragment_sender)?);
        }
        Ok(checks)
    }

    pub fn send_one(
        &mut self,
        option: u8,
        fragment_sender: &'a FragmentSender,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        match option % 10 {
            0 => fragment_sender.send_transaction(
                &mut self.sender,
                self.receiver,
                self.node,
                1.into(),
            ),
            1 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.get(index).unwrap();

                fragment_sender.send_full_delegation(&mut self.sender, stake_pool, self.node)
            }
            2 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.get(index).unwrap();

                fragment_sender.send_split_delegation(
                    &mut self.sender,
                    &vec![(stake_pool, 1)],
                    self.node,
                )
            }
            3 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.get(index).unwrap();

                fragment_sender.send_owner_delegation(&mut self.sender, stake_pool, self.node)
            }
            4 => {
                let stake_pool = StakePool::new(self.sender);
                self.active_stake_pools.push(stake_pool.clone());
                fragment_sender.send_pool_registration(&mut self.sender, &stake_pool, self.node)
            }
            5 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.get(index).unwrap();
                fragment_sender.send_pool_update(
                    &mut self.sender,
                    stake_pool,
                    stake_pool,
                    self.node,
                )
            }
            6 => {
                let index = self.rand.next_u32() as usize % self.active_stake_pools.len();
                let stake_pool = self.active_stake_pools.remove(index);

                fragment_sender.send_pool_retire(&mut self.sender, &stake_pool, self.node)
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
                self.vote_plans_for_tally.push(vote_plan.clone());
                fragment_sender.send_vote_plan(&mut self.sender, &vote_plan, self.node)
            }
            8 => fragment_sender.send_vote_cast(
                &mut self.sender,
                &self.vote_plan_for_casting,
                0,
                &Choice::new(1),
                self.node,
            ),
            /*9 => {
                let index = self.rand.next_u32() as usize % self.vote_plans_for_tally.len();
                let vote_plan = self.vote_plans_for_tally.remove(index);

                fragment_sender
                    .send_vote_tally(&mut self.sender, &vote_plan, self.node)
            },*/
            _ => unreachable!(),
        }
    }
}
