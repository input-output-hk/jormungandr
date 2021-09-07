use super::{
    chain_storable::{
        Address, BlockDate, BlockId, ChainLength, ExplorerVoteProposal, FragmentId, PoolId,
        PrivateVoteCast, ProposalId, PublicVoteCast, StorableHash, TransactionCertificate,
        TransactionInput, TransactionOutput, VotePlanId, VotePlanMeta,
    },
    endian::{B32, L32, L64},
    error::ExplorerError,
    helpers::open_or_create_db,
    pagination::{
        BlockFragmentsIter, BlocksInBranch, FragmentContentId, FragmentInputIter,
        FragmentOutputIter, SanakirjaCursorIter, TxsByAddress, VotePlanProposalsIter,
    },
    pair::Pair,
    state_ref::SerializedStateRef,
    Db, Pristine, SanakirjaTx, P,
};
use crate::db::state_ref::StateRef;
use chain_core::property::Fragment as _;
use chain_impl_mockchain::{
    certificate::Certificate,
    config::ConfigParam,
    fragment::Fragment,
    transaction::{self, InputEnum, Witness},
    value::Value,
};
use sanakirja::{
    btree::{self, UDb},
    direct_repr, Commit, RootDb, Storable, UnsizedStorable,
};
use std::convert::TryFrom;
use std::{convert::TryInto, sync::Arc};
use tracing::{debug, span, Level};
use zerocopy::{AsBytes, FromBytes};

pub type Txn = GenericTxn<::sanakirja::Txn<Arc<::sanakirja::Env>>>;
pub type MutTxn<T> = GenericTxn<::sanakirja::MutTxn<Arc<::sanakirja::Env>, T>>;

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(usize)]
pub enum Root {
    Stability,
    BooleanStaticSettings,
    Blocks,
    BlockTransactions,
    VotePlans,
    VotePlanProposals,
    TransactionInputs,
    TransactionOutputs,
    TransactionCertificates,
    TransactionBlocks,
    ChainLenghts,
    Tips,
    StakePoolData,
    States,
}

#[derive(Debug, Clone, AsBytes, FromBytes, PartialEq, Eq)]
#[repr(C)]
pub struct BranchHead {
    chain_length: B32,
    id: BlockId,
}

impl PartialOrd for BranchHead {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.chain_length.partial_cmp(&self.chain_length)
    }
}

impl Ord for BranchHead {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (other.chain_length, &other.id).cmp(&(self.chain_length, &self.id))
    }
}

direct_repr!(BranchHead);

pub type TransactionsInputs = Db<Pair<FragmentId, u8>, TransactionInput>;
pub type TransactionsOutputs = Db<Pair<FragmentId, u8>, TransactionOutput>;
pub type TransactionsCertificate = UDb<FragmentId, TransactionCertificate>;
pub type TransactionsBlocks = Db<FragmentId, BlockId>;
pub type Blocks = Db<BlockId, BlockMeta>;
pub type BlockTransactions = Db<BlockId, Pair<u8, FragmentId>>;
pub type ChainLengths = Db<ChainLength, BlockId>;
pub type ChainLengthsCursor = btree::Cursor<ChainLength, BlockId, P<ChainLength, BlockId>>;
pub type VotePlans = Db<VotePlanId, VotePlanMeta>;
pub type VotePlanProposals = Db<Pair<VotePlanId, u8>, ExplorerVoteProposal>;
pub type StakePools = Db<PoolId, StakePoolMeta>;
pub type Tips = Db<u8, BranchHead>;
pub type TipsCursor = btree::Cursor<u8, BranchHead, P<u8, BranchHead>>;

// multiverse
pub type States = Db<BlockId, SerializedStateRef>;

#[derive(Debug, AsBytes, FromBytes)]
#[repr(C)]
pub struct Stability {
    epoch_stability_depth: L32,
    last_stable_block: ChainLength,
}

#[derive(Debug, AsBytes, FromBytes)]
#[repr(C)]
pub struct StaticSettings {
    discrimination: L32,
    consensus: L32,
}

impl Pristine {
    pub fn txn_begin(&self) -> Result<Txn, ExplorerError> {
        let txn = ::sanakirja::Env::txn_begin(self.env.clone())?;
        fn begin(txn: ::sanakirja::Txn<Arc<::sanakirja::Env>>) -> Option<Txn> {
            Some(Txn {
                states: txn.root_db(Root::States as usize)?,
                tips: txn.root_db(Root::Tips as usize)?,
                chain_lengths: txn.root_db(Root::ChainLenghts as usize)?,
                transaction_inputs: txn.root_db(Root::TransactionInputs as usize)?,
                transaction_outputs: txn.root_db(Root::TransactionOutputs as usize)?,
                transaction_certificates: txn.root_db(Root::TransactionCertificates as usize)?,
                transaction_blocks: txn.root_db(Root::TransactionBlocks as usize)?,
                blocks: txn.root_db(Root::Blocks as usize)?,
                block_transactions: txn.root_db(Root::BlockTransactions as usize)?,
                vote_plans: txn.root_db(Root::VotePlans as usize)?,
                vote_plan_proposals: txn.root_db(Root::VotePlanProposals as usize)?,
                stake_pool_data: txn.root_db(Root::StakePoolData as usize)?,
                txn,
            })
        }
        if let Some(txn) = begin(txn) {
            Ok(txn)
        } else {
            Err(ExplorerError::UnitializedDatabase)
        }
    }

    pub fn mut_txn_begin(&self) -> Result<MutTxn<()>, ExplorerError> {
        let mut txn = ::sanakirja::Env::mut_txn_begin(self.env.clone()).unwrap();
        Ok(MutTxn {
            states: open_or_create_db(&mut txn, Root::States)?,
            tips: open_or_create_db(&mut txn, Root::Tips)?,
            chain_lengths: open_or_create_db(&mut txn, Root::ChainLenghts)?,
            transaction_inputs: open_or_create_db(&mut txn, Root::TransactionInputs)?,
            transaction_outputs: open_or_create_db(&mut txn, Root::TransactionOutputs)?,
            transaction_certificates: open_or_create_db(&mut txn, Root::TransactionCertificates)?,
            transaction_blocks: open_or_create_db(&mut txn, Root::TransactionBlocks)?,
            blocks: open_or_create_db(&mut txn, Root::Blocks)?,
            block_transactions: open_or_create_db(&mut txn, Root::BlockTransactions)?,
            vote_plans: open_or_create_db(&mut txn, Root::VotePlans)?,
            vote_plan_proposals: open_or_create_db(&mut txn, Root::VotePlanProposals)?,
            stake_pool_data: open_or_create_db(&mut txn, Root::StakePoolData)?,
            txn,
        })
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[repr(C)]
pub struct StakePoolMeta {
    pub registration: FragmentId,
    pub retirement: Option<FragmentId>,
}

direct_repr!(StakePoolMeta);

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
#[repr(C)]
pub struct BlockMeta {
    pub chain_length: ChainLength,
    pub date: BlockDate,
    pub parent_hash: BlockId,
}

direct_repr!(BlockMeta);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct BlockProducer {
    bytes: [u8; 32],
}

direct_repr!(BlockProducer);

pub struct GenericTxn<T: ::sanakirja::LoadPage<Error = ::sanakirja::Error> + ::sanakirja::RootPage>
{
    #[doc(hidden)]
    pub txn: T,

    pub states: States,
    pub tips: Tips,
    pub chain_lengths: ChainLengths,
    pub transaction_inputs: TransactionsInputs,
    pub transaction_outputs: TransactionsOutputs,
    pub transaction_certificates: TransactionsCertificate,
    pub transaction_blocks: TransactionsBlocks,
    pub blocks: Blocks,
    pub block_transactions: BlockTransactions,
    pub vote_plans: VotePlans,
    pub vote_plan_proposals: VotePlanProposals,
    pub stake_pool_data: StakePools,
}

impl<T: ::sanakirja::LoadPage<Error = ::sanakirja::Error> + ::sanakirja::RootPage> GenericTxn<T> {}

impl MutTxn<()> {
    pub fn add_block0<'a>(
        &mut self,
        parent_id: &BlockId,
        block0_id: &BlockId,
        fragments: impl Iterator<Item = &'a Fragment>,
    ) -> Result<(), ExplorerError> {
        let span = span!(Level::DEBUG, "add_block0");
        let _enter = span.enter();
        debug!(?parent_id, ?block0_id);

        let state_ref = StateRef::new_empty(&mut self.txn);

        unsafe {
            self.txn.set_root(
                Root::Stability as usize,
                std::mem::transmute(Stability::default()),
            )
        };

        let tip = BranchHead {
            chain_length: B32::new(0),
            id: block0_id.clone(),
        };

        assert!(btree::put(&mut self.txn, &mut self.tips, &1, &tip)?);

        // the tip is also the only branch
        let head = BranchHead {
            chain_length: B32::new(0),
            id: block0_id.clone(),
        };

        assert!(btree::put(&mut self.txn, &mut self.tips, &0, &head)?);

        self.update_state(
            fragments,
            state_ref,
            ChainLength::new(0),
            &block0_id,
            BlockDate {
                epoch: B32::new(0),
                slot_id: B32::new(0),
            },
            &parent_id,
        )?;

        Ok(())
    }

    pub fn add_block<'a>(
        &mut self,
        parent_id: &BlockId,
        block_id: &BlockId,
        chain_length: ChainLength,
        block_date: BlockDate,
        fragments: impl IntoIterator<Item = &'a Fragment>,
    ) -> Result<(), ExplorerError> {
        let span = span!(Level::DEBUG, "add_block");
        let _enter = span.enter();
        debug!(?parent_id, ?block_id, ?chain_length);

        self.update_tips(&parent_id, chain_length, &block_id)?;

        let states = btree::get(&self.txn, &self.states, &parent_id, None)?
            .filter(|(branch_id, _states)| *branch_id == parent_id)
            .map(|(_branch_id, states)| states)
            .cloned()
            .ok_or_else(|| {
                ExplorerError::AncestorNotFound(block_id.clone().into(), parent_id.clone().into())
            })?;

        debug!("forking states");

        let state_ref = states.fork(&mut self.txn);

        self.update_state(
            fragments.into_iter(),
            state_ref,
            chain_length,
            &block_id,
            block_date,
            parent_id,
        )?;

        Ok(())
    }

    pub fn set_tip(&mut self, id: BlockId) -> Result<bool, ExplorerError> {
        let block_meta = btree::get(&self.txn, &self.blocks, &id, None)?.filter(|(k, _)| *k == &id);

        if let Some(block_meta) = block_meta.map(|(_, meta)| meta.clone()) {
            assert!(btree::del(&mut self.txn, &mut self.tips, &0, None)?);

            let new_tip = BranchHead {
                id,
                chain_length: block_meta.chain_length.0,
            };

            assert!(btree::put(&mut self.txn, &mut self.tips, &0, &new_tip)?);

            self.gc_stable_states(block_meta.chain_length.get())?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// this drops old states at: tip_chain_length - (epoch_stability_depth + 1) so we keep the
    /// amount of forks bounded, because we don't need to fork those states anymore.
    fn gc_stable_states(&mut self, tip_chain_length: u32) -> Result<(), ExplorerError> {
        let mut stability: Stability =
            unsafe { std::mem::transmute(self.txn.root(Root::Stability as usize).unwrap()) };
        if let Some(new_stable_chain_length) =
            tip_chain_length.checked_sub(stability.epoch_stability_depth.get())
        {
            stability.last_stable_block = ChainLength::new(new_stable_chain_length);
        }
        if let Some(collect_at) = stability.last_stable_block.get().checked_sub(1) {
            let mut states_to_gc = vec![];

            let iter = btree::iter(
                &self.txn,
                &self.chain_lengths,
                Some((&ChainLength::new(collect_at), None)),
            )?;

            for block in iter {
                let (chain_length, block_id) = block?;

                if chain_length.get() > collect_at {
                    break;
                }

                let states = btree::get(&self.txn, &self.states, &block_id, None)?
                    .filter(|(branch_id, _states)| *branch_id == block_id)
                    .map(|(_branch_id, states)| StateRef::from(states.clone()));

                states_to_gc.push((block_id.clone(), states.unwrap()));
            }

            for (block_id, state) in states_to_gc.drain(..) {
                // this is safe because after dropping we are inmediately deleting the state from
                // `self.states`.
                unsafe {
                    state.drop(&mut self.txn)?;
                    btree::del(&mut self.txn, &mut self.states, &block_id, None)?;
                }
                btree::del(
                    &mut self.txn,
                    &mut self.chain_lengths,
                    &ChainLength::new(collect_at),
                    Some(&block_id),
                )?;

                // TODO: there is more gc to do here.  We need to remove the transactions in the
                // block we are removing from the global indices, but only if the block is not
                // included in the main-branch (e.g. is not confirmed).
                //
                // We can get the block, ask if it's confirmed.
                // If it isn't, for each fragment, ask if it is present in any other block, and if it isn't, delete it.
                //
                // This is not top priority because that data will only be reachable by id. So it
                // could be done by a separate task in intervals maybe.
            }
        }
        unsafe {
            self.txn
                .set_root(Root::Stability as usize, std::mem::transmute(stability))
        };
        Ok(())
    }

    fn update_state<'a>(
        &mut self,
        fragments: impl Iterator<Item = &'a Fragment>,
        mut state_ref: StateRef,
        chain_length: ChainLength,
        block_id: &StorableHash,
        block_date: BlockDate,
        parent_id: &StorableHash,
    ) -> Result<(), ExplorerError> {
        let span = span!(Level::DEBUG, "update_state");
        let _enter = span.enter();
        debug!(?block_id);

        self.apply_fragments(&block_id, fragments, &mut state_ref)?;
        state_ref.add_block_to_blocks(&mut self.txn, &chain_length, &block_id)?;

        let new_state = state_ref.finish(&mut self.txn);

        debug!("adding new state");
        if !btree::put(&mut self.txn, &mut self.states, &block_id, &new_state)? {
            return Err(ExplorerError::BlockAlreadyExists(block_id.clone().into()));
        }

        self.update_chain_lengths(chain_length, &block_id)?;

        self.add_block_meta(
            block_id,
            BlockMeta {
                chain_length,
                date: block_date,
                parent_hash: parent_id.clone(),
            },
        )?;

        Ok(())
    }

    fn apply_fragments<'a>(
        &mut self,
        block_id: &BlockId,
        fragments: impl Iterator<Item = &'a Fragment>,
        state_ref: &mut StateRef,
    ) -> Result<(), ExplorerError> {
        for (idx, fragment) in fragments.enumerate() {
            let fragment_id = StorableHash::from(fragment.id());

            btree::put(
                &mut self.txn,
                &mut self.block_transactions,
                &block_id,
                &Pair {
                    a: u8::try_from(idx).expect("found more than 255 fragments in a block"),
                    b: fragment_id.clone(),
                },
            )?;

            btree::put(
                &mut self.txn,
                &mut self.transaction_blocks,
                &fragment_id,
                &block_id,
            )?;

            match &fragment {
                Fragment::Initial(config_params) => {
                    let mut settings = StaticSettings::new();
                    let mut stability: Stability = unsafe {
                        std::mem::transmute(self.txn.root(Root::Stability as usize).unwrap())
                    };

                    for config_param in config_params.iter() {
                        match config_param {
                            ConfigParam::Discrimination(d) => {
                                settings.set_discrimination(*d);
                            }
                            ConfigParam::Block0Date(_) => {}
                            ConfigParam::ConsensusVersion(c) => {
                                settings.set_consensus(*c);
                            }
                            ConfigParam::SlotsPerEpoch(_) => {}
                            ConfigParam::SlotDuration(_) => {}
                            ConfigParam::EpochStabilityDepth(c) => {
                                stability.set_epoch_stability_depth(*c);
                            }
                            ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(_) => {}
                            ConfigParam::BlockContentMaxSize(_) => {}
                            ConfigParam::AddBftLeader(_) => {}
                            ConfigParam::RemoveBftLeader(_) => {}
                            ConfigParam::LinearFee(_) => {}
                            ConfigParam::ProposalExpiration(_) => {}
                            ConfigParam::KesUpdateSpeed(_) => {}
                            ConfigParam::TreasuryAdd(_) => {}
                            ConfigParam::TreasuryParams(_) => {}
                            ConfigParam::RewardPot(_) => {}
                            ConfigParam::RewardParams(_) => {}
                            ConfigParam::PerCertificateFees(_) => {}
                            ConfigParam::FeesInTreasury(_) => {}
                            ConfigParam::RewardLimitNone => {}
                            ConfigParam::RewardLimitByAbsoluteStake(_) => {}
                            ConfigParam::PoolRewardParticipationCapping(_) => {}
                            ConfigParam::AddCommitteeId(_) => {}
                            ConfigParam::RemoveCommitteeId(_) => {}
                            ConfigParam::PerVoteCertificateFees(_) => {}
                        }
                    }

                    unsafe {
                        self.txn
                            .set_root(Root::Stability as usize, std::mem::transmute(stability));
                        self.txn.set_root(
                            Root::BooleanStaticSettings as usize,
                            std::mem::transmute(settings),
                        );
                    }
                }
                Fragment::OldUtxoDeclaration(_) => {}
                Fragment::Transaction(tx) => {
                    self.apply_transaction(fragment_id, &tx, state_ref)?;
                }
                Fragment::OwnerStakeDelegation(tx) => {
                    self.apply_transaction(fragment_id, &tx, state_ref)?;
                }
                Fragment::StakeDelegation(tx) => {
                    self.apply_transaction(fragment_id, &tx, state_ref)?;
                }
                Fragment::PoolRegistration(tx) => {
                    self.apply_transaction(fragment_id, &tx, state_ref)?;
                }
                Fragment::PoolRetirement(tx) => {
                    self.apply_transaction(fragment_id, &tx, state_ref)?;
                }
                Fragment::PoolUpdate(tx) => {
                    self.apply_transaction(fragment_id, &tx, state_ref)?;
                }
                Fragment::UpdateProposal(_) => {}
                Fragment::UpdateVote(_) => {}
                Fragment::VotePlan(tx) => {
                    self.apply_transaction(fragment_id.clone(), &tx, state_ref)?;
                    self.add_vote_plan(&fragment_id, tx)?;
                }
                Fragment::VoteCast(tx) => {
                    self.apply_transaction(fragment_id.clone(), &tx, state_ref)?;

                    let vote_cast = tx.as_slice().payload().into_payload();
                    let vote_plan_id =
                        StorableHash::from(<[u8; 32]>::from(vote_cast.vote_plan().clone()));
                    let proposal_index = vote_cast.proposal_index();

                    let proposal_id = ProposalId {
                        vote_plan: vote_plan_id.clone(),
                        index: proposal_index,
                    };

                    state_ref.apply_vote(&mut self.txn, &fragment_id, &proposal_id)?;

                    let certificate = match vote_cast.payload() {
                        chain_impl_mockchain::vote::Payload::Public { choice } => {
                            let vote_cast = PublicVoteCast {
                                vote_plan_id,
                                proposal_index,
                                choice: choice.as_byte(),
                            };

                            TransactionCertificate::from_public_vote_cast(vote_cast)
                        }

                        // private vote not supported yet
                        chain_impl_mockchain::vote::Payload::Private {
                            encrypted_vote: _,
                            proof: _,
                        } => {
                            let vote_cast = PrivateVoteCast {
                                vote_plan_id,
                                proposal_index,
                            };

                            TransactionCertificate::from_private_vote_cast(vote_cast)
                        }
                    };

                    btree::put(
                        &mut self.txn,
                        &mut self.transaction_certificates,
                        &fragment_id,
                        &certificate,
                    )?;
                }
                Fragment::VoteTally(tx) => {
                    self.apply_transaction(fragment_id, &tx, state_ref)?;
                }
                Fragment::EncryptedVoteTally(tx) => {
                    self.apply_transaction(fragment_id, &tx, state_ref)?;
                }
            }

            self.update_stake_pool_meta(&fragment)?;
        }

        Ok(())
    }

    fn get_settings(&self) -> StaticSettings {
        let raw = self.txn.root(Root::BooleanStaticSettings as usize).unwrap();

        unsafe { std::mem::transmute(raw) }
    }

    fn add_vote_plan(
        &mut self,
        fragment_id: &FragmentId,
        tx: &transaction::Transaction<chain_impl_mockchain::certificate::VotePlan>,
    ) -> Result<(), ExplorerError> {
        let vote_plan = tx.as_slice().payload().into_payload();
        let vote_plan_id = StorableHash::from(<[u8; 32]>::from(vote_plan.to_id()));
        let vote_plan_meta = VotePlanMeta {
            vote_start: vote_plan.vote_start().into(),
            vote_end: vote_plan.vote_end().into(),
            committee_end: vote_plan.committee_end().into(),
            payload_type: vote_plan.payload_type().into(),
        };

        for (idx, proposal) in vote_plan.proposals().iter().enumerate() {
            btree::put(
                &mut self.txn,
                &mut self.vote_plan_proposals,
                &Pair {
                    a: vote_plan_id.clone(),
                    b: idx as u8,
                },
                &proposal.into(),
            )?;
        }

        btree::put(
            &mut self.txn,
            &mut self.vote_plans,
            &vote_plan_id,
            &vote_plan_meta,
        )?;

        btree::put(
            &mut self.txn,
            &mut self.transaction_certificates,
            &fragment_id,
            &TransactionCertificate::from_vote_plan_id(vote_plan_id),
        )?;

        Ok(())
    }

    fn apply_transaction<P>(
        &mut self,
        fragment_id: FragmentId,
        tx: &transaction::Transaction<P>,
        state: &mut StateRef,
    ) -> Result<(), ExplorerError> {
        let etx = tx.as_slice();

        // is important that we put outputs first, because utxo's can refer to inputs in the same
        // transaction, so those need to be already indexed. Although it would be technically
        // faster to just look for them in the serialized transaction, that's increases complexity
        // for something that is not really that likely. Besides, the pages should be in the system
        // cache because we recently inserted them.
        for (idx, output) in etx.outputs().iter().enumerate() {
            self.put_transaction_output(fragment_id.clone(), idx as u8, &output)?;
            state.apply_output_to_stake_control(&mut self.txn, &output)?;
            state.add_transaction_to_address(
                &mut self.txn,
                &fragment_id,
                &output.address.into(),
            )?;
        }

        for (index, (input, witness)) in etx.inputs_and_witnesses().iter().enumerate() {
            self.put_transaction_input(fragment_id.clone(), index as u8, &input, &witness)?;

            let resolved_utxo = match input.to_enum() {
                InputEnum::AccountInput(_, _) => None,
                InputEnum::UtxoInput(input) => {
                    Some(self.resolve_utxo(&self.transaction_outputs, input)?.clone())
                }
            };

            self.apply_input_to_stake_control(&input, &witness, resolved_utxo.as_ref(), state)?;

            self.apply_input_to_transactions_by_address(
                &fragment_id,
                &input,
                &witness,
                resolved_utxo.as_ref(),
                state,
            )?;
        }

        Ok(())
    }

    fn update_tips(
        &mut self,
        parent_id: &BlockId,
        chain_length: ChainLength,
        block_id: &BlockId,
    ) -> Result<(), ExplorerError> {
        let span = span!(Level::DEBUG, "update_tips");
        let _enter = span.enter();

        debug!(?parent_id, ?chain_length, ?block_id);

        let parent_key = BranchHead {
            chain_length: B32::new(
                chain_length
                    .0
                    .get()
                    .checked_sub(1)
                    .expect("update tips called with block0"),
            ),
            id: parent_id.clone(),
        };

        debug!(?parent_key);

        let _ = btree::del(&mut self.txn, &mut self.tips, &1, Some(&parent_key))?;

        let key = BranchHead {
            chain_length: chain_length.0,
            id: block_id.clone(),
        };

        debug!(?key);

        btree::put(&mut self.txn, &mut self.tips, &1, &key)?;

        Ok(())
    }

    fn update_chain_lengths(
        &mut self,
        chain_length: ChainLength,
        block_id: &BlockId,
    ) -> Result<(), ExplorerError> {
        btree::put(
            &mut self.txn,
            &mut self.chain_lengths,
            &chain_length,
            block_id,
        )?;

        Ok(())
    }

    fn put_transaction_input(
        &mut self,
        fragment_id: FragmentId,
        index: u8,
        input: &transaction::Input,
        witness: &transaction::Witness,
    ) -> Result<(), ExplorerError> {
        btree::put(
            &mut self.txn,
            &mut self.transaction_inputs,
            &Pair {
                a: fragment_id,
                b: index,
            },
            &TransactionInput::from_original_with_witness(input, witness),
        )?;

        Ok(())
    }

    fn put_transaction_output(
        &mut self,
        fragment_id: FragmentId,
        index: u8,
        output: &transaction::Output<chain_addr::Address>,
    ) -> Result<(), ExplorerError> {
        btree::put(
            &mut self.txn,
            &mut self.transaction_outputs,
            &Pair {
                a: fragment_id,
                b: index,
            },
            &TransactionOutput::from_original(output),
        )?;

        Ok(())
    }

    fn update_stake_pool_meta(&mut self, fragment: &Fragment) -> Result<(), ExplorerError> {
        match fragment {
            Fragment::PoolRegistration(tx) => {
                let etx = tx.as_slice();
                let cert = etx.payload();

                let stake_pool_id = match cert.into_certificate_slice().unwrap().into_owned() {
                    Certificate::PoolRegistration(r) => r.to_id(),
                    _ => unreachable!("mismatched certificate type"),
                };

                btree::put(
                    &mut self.txn,
                    &mut self.stake_pool_data,
                    &StorableHash::from(<[u8; 32]>::from(stake_pool_id)),
                    &StakePoolMeta {
                        registration: StorableHash::from(fragment.id()),
                        retirement: None,
                    },
                )?;
            }
            Fragment::PoolRetirement(tx) => {
                let etx = tx.as_slice();
                let cert = etx.payload();

                let stake_pool_id = match cert.into_certificate_slice().unwrap().into_owned() {
                    Certificate::PoolRetirement(r) => r.pool_id,
                    _ => unreachable!("mismatched certificate type"),
                };

                let stake_pool_id = StorableHash::from(<[u8; 32]>::from(stake_pool_id));

                let mut new = btree::get(&self.txn, &self.stake_pool_data, &stake_pool_id, None)?
                    .map(|(_, meta)| meta)
                    .cloned()
                    .unwrap();

                new.retirement = Some(FragmentId::from(fragment.id()));

                btree::del(
                    &mut self.txn,
                    &mut self.stake_pool_data,
                    &stake_pool_id,
                    None,
                )?;

                btree::put(
                    &mut self.txn,
                    &mut self.stake_pool_data,
                    &stake_pool_id,
                    &new,
                )?;
            }
            _ => {}
        }

        Ok(())
    }

    fn add_block_meta(
        &mut self,
        block_id: &BlockId,
        block: BlockMeta,
    ) -> Result<(), ExplorerError> {
        btree::put(&mut self.txn, &mut self.blocks, block_id, &block)?;

        Ok(())
    }

    fn apply_input_to_stake_control(
        &mut self,
        input: &transaction::Input,
        witness: &transaction::Witness,
        resolved_utxo: Option<&TransactionOutput>,
        state: &mut StateRef,
    ) -> Result<(), ExplorerError> {
        match (input.to_enum(), witness) {
            (InputEnum::AccountInput(account, value), Witness::Account(_)) => {
                state.substract_stake_from_account(
                    &mut self.txn,
                    account.to_single_account().unwrap().as_ref(),
                    value,
                );
            }
            (InputEnum::AccountInput(_, _), Witness::Multisig(_)) => {}
            (InputEnum::UtxoInput(_), Witness::Utxo(_)) => {
                // TODO: this is not the cleanest way of doing this...
                let output = resolved_utxo.expect("missing utxo pointer resolution");

                let address: chain_addr::Address = output.address.clone().try_into().unwrap();

                if let chain_addr::Kind::Group(_, account) = address.kind() {
                    let value = &output.value;
                    state.substract_stake_from_account(&mut self.txn, &account, Value(value.get()));
                }
            }
            (InputEnum::UtxoInput(_), Witness::OldUtxo(_, _, _)) => {}
            _ => unreachable!("invalid combination of input and witness"),
        }
        Ok(())
    }

    fn apply_input_to_transactions_by_address(
        &mut self,
        fragment_id: &super::chain_storable::FragmentId,
        input: &transaction::Input,
        witness: &transaction::Witness,
        resolved_utxo: Option<&TransactionOutput>,
        state: &mut StateRef,
    ) -> Result<(), ExplorerError> {
        match (input.to_enum(), witness) {
            (InputEnum::AccountInput(account_id, _value), Witness::Account(_)) => {
                let kind = chain_addr::Kind::Account(
                    account_id
                        .to_single_account()
                        .expect("the input to be validated")
                        .into(),
                );

                let discrimination = self.get_settings().get_discrimination().unwrap();
                let address = chain_addr::Address(discrimination, kind).into();

                state.add_transaction_to_address(&mut self.txn, &fragment_id, &address)?;
            }
            (InputEnum::AccountInput(_, _), Witness::Multisig(_)) => {}
            (InputEnum::UtxoInput(_), Witness::Utxo(_)) => {
                // TODO: this is not the cleanest way of doing this...
                let output = resolved_utxo.expect("missing utxo pointer resolution");

                state.add_transaction_to_address(
                    &mut self.txn,
                    &fragment_id,
                    &output.address.clone(),
                )?;
            }
            (InputEnum::UtxoInput(_), Witness::OldUtxo(_, _, _)) => {}
            _ => unreachable!("invalid combination of input and witness"),
        }

        Ok(())
    }

    // mostly used to retrieve the address of a utxo input (because it's embedded in the output)
    // we need this mostly to know the addresses involved in a tx.
    // but it is also used for stake/funds tracking, as we need to know how much to substract.
    fn resolve_utxo(
        &self,
        transactions: &TransactionsOutputs,
        utxo_pointer: transaction::UtxoPointer,
    ) -> Result<&TransactionOutput, ExplorerError> {
        let txid = utxo_pointer.transaction_id;
        let idx = utxo_pointer.output_index;

        let mut cursor = btree::Cursor::new(&self.txn, &transactions)?;

        let key = Pair {
            a: StorableHash::from(txid),
            b: idx,
        };

        cursor.set(
            &self.txn,
            &key,
            Some(&TransactionOutput {
                // address: Address::MAX,
                address: Address::MIN,
                value: L64::new(u64::MIN),
            }),
        )?;

        if let Some((_, output)) = cursor.current(&self.txn)?.filter(|(k, _)| *k == &key) {
            Ok(output)
        } else {
            panic!("missing utxo {:?}", txid)
        }
    }

    pub fn commit(self) -> Result<(), ExplorerError> {
        // destructure things so we get some sort of exhaustiveness-check
        let Self {
            mut txn,
            states,
            tips,
            chain_lengths,
            transaction_inputs,
            transaction_outputs,
            transaction_certificates,
            transaction_blocks,
            blocks,
            block_transactions,
            vote_plans,
            vote_plan_proposals,
            stake_pool_data,
        } = self;

        txn.set_root(Root::States as usize, states.db);
        txn.set_root(Root::Tips as usize, tips.db);
        txn.set_root(Root::ChainLenghts as usize, chain_lengths.db);
        txn.set_root(Root::TransactionInputs as usize, transaction_inputs.db);
        txn.set_root(Root::TransactionOutputs as usize, transaction_outputs.db);
        txn.set_root(
            Root::TransactionCertificates as usize,
            transaction_certificates.db,
        );
        txn.set_root(Root::TransactionBlocks as usize, transaction_blocks.db);
        txn.set_root(Root::Blocks as usize, blocks.db);
        txn.set_root(Root::BlockTransactions as usize, block_transactions.db);
        txn.set_root(Root::VotePlans as usize, vote_plans.db);
        txn.set_root(Root::VotePlanProposals as usize, vote_plan_proposals.db);
        txn.set_root(Root::StakePoolData as usize, stake_pool_data.db);

        txn.commit()?;

        Ok(())
    }
}

impl Txn {
    pub fn get_transactions_by_address<'a>(
        &'a self,
        state_id: &StorableHash,
        address: &Address,
    ) -> Result<Option<TxsByAddress<'a>>, ExplorerError> {
        let state = btree::get(&self.txn, &self.states, &state_id, None)?;

        let state = match state {
            Some((s, state)) if state_id == s => StateRef::from(state.clone()),
            _ => return Ok(None),
        };

        let address_id = match btree::get(&self.txn, &state.address_id, &address, None)? {
            Some((a, id)) if a == address => id,
            _ => return Ok(None),
        };

        Ok(Some(SanakirjaCursorIter::new(
            &self.txn,
            address_id.into(),
            &state.address_transactions,
        )?))
    }

    pub fn get_blocks<'a>(
        &'a self,
        state_id: &StorableHash,
    ) -> Result<Option<BlocksInBranch>, ExplorerError> {
        let state = btree::get(&self.txn, &self.states, &state_id, None)?;

        let state = match state {
            Some((s, state)) if state_id == s => StateRef::from(state.clone()),
            _ => return Ok(None),
        };

        Ok(Some(SanakirjaCursorIter::new(
            &self.txn,
            (),
            &state.blocks,
        )?))
    }

    pub fn get_branches(&self) -> Result<BranchIter, ExplorerError> {
        let mut cursor = btree::Cursor::new(&self.txn, &self.tips)?;

        // skip the tip tag
        cursor.next(&self.txn)?;

        Ok(BranchIter {
            txn: &self.txn,
            cursor,
        })
    }

    pub fn get_tip(&self) -> Result<BlockId, ExplorerError> {
        let mut cursor = btree::Cursor::new(&self.txn, &self.tips)?;

        Ok(cursor.next(&self.txn)?.unwrap().1.id.clone())
    }

    pub fn get_block_fragments<'a, 'b: 'a>(
        &'a self,
        block_id: &'b BlockId,
    ) -> Result<BlockFragmentsIter, ExplorerError> {
        SanakirjaCursorIter::new(&self.txn, block_id.into(), &self.block_transactions)
    }

    pub fn get_fragment_inputs<'a, 'b: 'a>(
        &'a self,
        fragment_id: &'b FragmentId,
    ) -> Result<FragmentInputIter<'a>, ExplorerError> {
        SanakirjaCursorIter::new(
            &self.txn,
            FragmentContentId::from(fragment_id),
            &self.transaction_inputs,
        )
    }

    pub fn get_fragment_outputs<'a, 'b: 'a>(
        &'a self,
        fragment_id: &'b FragmentId,
    ) -> Result<FragmentOutputIter<'a>, ExplorerError> {
        SanakirjaCursorIter::new(
            &self.txn,
            FragmentContentId::from(fragment_id),
            &self.transaction_outputs,
        )
    }

    pub fn get_fragment_certificate(
        &self,
        fragment_id: &FragmentId,
    ) -> Result<Option<&TransactionCertificate>, ExplorerError> {
        let key = fragment_id.clone();

        let certificate = btree::get(&self.txn, &self.transaction_certificates, &key, None)?;

        Ok(certificate.and_then(|(k, v)| if k == &key { Some(v) } else { None }))
    }

    pub fn get_blocks_by_chain_length<'a, 'b: 'a>(
        &'a self,
        chain_length: &'b ChainLength,
    ) -> Result<impl Iterator<Item = Result<&'a BlockId, ExplorerError>>, ExplorerError> {
        let mut cursor = btree::Cursor::new(&self.txn, &self.chain_lengths)?;

        cursor.set(&self.txn, chain_length, None)?;

        Ok(BlocksByChainLenght {
            txn: &self.txn,
            cursor,
            chain_length: *chain_length,
        })
    }

    pub fn get_block_meta(&self, block_id: &BlockId) -> Result<Option<&BlockMeta>, ExplorerError> {
        let block_meta = btree::get(&self.txn, &self.blocks, &block_id, None)?;

        Ok(block_meta.and_then(|(k, v)| if k == block_id { Some(v) } else { None }))
    }

    pub fn get_vote_plan_meta(
        &self,
        vote_plan_id: &VotePlanId,
    ) -> Result<Option<&VotePlanMeta>, ExplorerError> {
        let certificate = btree::get(&self.txn, &self.vote_plans, &vote_plan_id, None)?;

        Ok(certificate.and_then(|(k, v)| if k == vote_plan_id { Some(v) } else { None }))
    }

    pub fn get_vote_plan_proposals<'a, 'b: 'a>(
        &'a self,
        vote_plan_id: &'b VotePlanId,
    ) -> Result<VotePlanProposalsIter, ExplorerError> {
        SanakirjaCursorIter::new(&self.txn, vote_plan_id.into(), &self.vote_plan_proposals)
    }

    // TODO: this is duplicated in the MutTxn, it would be nice to find a way to re-use it, but I'm
    // not sure if there is any trait combination that allows it easily (for T).
    pub fn get_settings(&self) -> StaticSettings {
        let raw = self.txn.root(Root::BooleanStaticSettings as usize);

        unsafe { std::mem::transmute(raw) }
    }

    pub fn get_last_stable_block(&self) -> ChainLength {
        let stability: Stability =
            unsafe { std::mem::transmute(self.txn.root(Root::Stability as usize)) };

        stability.last_stable_block
    }

    // paginating this seems a bit overkill
    pub fn transaction_blocks(&self, tx: &FragmentId) -> Result<Vec<BlockId>, ExplorerError> {
        let iter = btree::iter(&self.txn, &self.transaction_blocks, Some((tx, None)))?;

        let mut ids = vec![];

        for block in iter {
            let (k, block) = block?;

            if tx != k {
                break;
            }

            ids.push(block.clone());
        }

        Ok(ids)
    }
}

pub struct BranchIter<'a> {
    txn: &'a SanakirjaTx,
    cursor: TipsCursor,
}

impl<'a> Iterator for BranchIter<'a> {
    type Item = Result<&'a FragmentId, ExplorerError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor
            .next(self.txn)
            .transpose()
            .map(|item| item.map(|(_, v)| &v.id).map_err(ExplorerError::from))
    }
}

pub struct BlocksByChainLenght<'a> {
    txn: &'a SanakirjaTx,
    cursor: ChainLengthsCursor,
    chain_length: ChainLength,
}

impl<'a> Iterator for BlocksByChainLenght<'a> {
    type Item = Result<&'a BlockId, ExplorerError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor
            .next(self.txn)
            .map(|item| {
                item.and_then(|(k, v)| {
                    if k == &self.chain_length {
                        Some(v)
                    } else {
                        None
                    }
                })
            })
            .map_err(ExplorerError::from)
            .transpose()
    }
}

impl Default for Stability {
    fn default() -> Self {
        Self {
            epoch_stability_depth: L32::new(u32::MAX),
            last_stable_block: ChainLength::new(0),
        }
    }
}

impl Stability {
    pub fn set_epoch_stability_depth(&mut self, e: u32) {
        self.epoch_stability_depth = L32::new(e);
    }

    pub fn get_epoch_stability_depth(&self) -> u32 {
        self.epoch_stability_depth.get()
    }
}

impl StaticSettings {
    pub fn new() -> Self {
        Self {
            discrimination: L32::new(0),
            consensus: L32::new(0),
        }
    }

    pub fn set_discrimination(&mut self, d: chain_addr::Discrimination) {
        match d {
            chain_addr::Discrimination::Production => self.discrimination = L32::new(1),
            chain_addr::Discrimination::Test => self.discrimination = L32::new(2),
        }
    }

    pub fn get_discrimination(&self) -> Option<chain_addr::Discrimination> {
        match self.discrimination.get() {
            0 => None,
            1 => Some(chain_addr::Discrimination::Production),
            2 => Some(chain_addr::Discrimination::Test),
            _ => unreachable!("invalid discrimination tag"),
        }
    }
    pub fn set_consensus(&mut self, c: chain_impl_mockchain::chaintypes::ConsensusType) {
        match c {
            chain_impl_mockchain::chaintypes::ConsensusType::Bft => self.consensus = L32::new(1),
            chain_impl_mockchain::chaintypes::ConsensusType::GenesisPraos => {
                self.consensus = L32::new(2)
            }
        }
    }

    pub fn get_consensus(&self) -> Option<chain_impl_mockchain::chaintypes::ConsensusType> {
        match self.consensus.get() {
            0 => None,
            1 => Some(chain_impl_mockchain::chaintypes::ConsensusType::Bft),
            2 => Some(chain_impl_mockchain::chaintypes::ConsensusType::GenesisPraos),
            _ => unreachable!("invalid discrimination tag"),
        }
    }
}

impl Default for StaticSettings {
    fn default() -> Self {
        Self::new()
    }
}
