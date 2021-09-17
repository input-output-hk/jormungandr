use super::{
    chain_storable::{
        BlockDate, BlockId, ChainLength, ExplorerVoteProposal, FragmentId, TransactionCertificate,
        TransactionInput, TransactionOutput, VotePlanId, VotePlanMeta,
    },
    endian::{B32, L32},
    error::DbError,
    helpers::open_or_create_db,
    pair::Pair,
    state_ref::SerializedStateRef,
    Db, ExplorerDb, P,
};
use chain_impl_mockchain::fragment::Fragment;
use sanakirja::{
    btree::{self, UDb},
    direct_repr, Commit, RootDb, Storable, UnsizedStorable,
};
use std::sync::Arc;
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
    States,
}

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
pub type Tips = Db<BranchTag, BranchHead>;

// multiverse
pub type States = Db<BlockId, SerializedStateRef>;

#[derive(Debug, Clone, AsBytes, FromBytes, PartialEq, Eq)]
#[repr(C)]
pub struct BranchHead {
    chain_length: B32,
    id: BlockId,
}

#[derive(Debug, Clone, Copy, AsBytes, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BranchTag {
    Tip = 0,
    Branch = 1,
}

impl PartialOrd for BranchHead {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // NOTE: the order is reversed, so branches are stored in descending order
        other.chain_length.partial_cmp(&self.chain_length)
    }
}

impl Ord for BranchHead {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (other.chain_length, &other.id).cmp(&(self.chain_length, &self.id))
    }
}

direct_repr!(BranchTag);
direct_repr!(BranchHead);

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

impl ExplorerDb {
    pub fn txn_begin(&self) -> Result<Txn, DbError> {
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
                txn,
            })
        }
        if let Some(txn) = begin(txn) {
            Ok(txn)
        } else {
            Err(DbError::UnitializedDatabase)
        }
    }

    pub fn mut_txn_begin(&self) -> Result<MutTxn<()>, DbError> {
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
}

impl<T: ::sanakirja::LoadPage<Error = ::sanakirja::Error> + ::sanakirja::RootPage> GenericTxn<T> {}

impl MutTxn<()> {
    pub fn add_block0<'a>(
        &mut self,
        _parent_id: &BlockId,
        _block0_id: &BlockId,
        _fragments: impl Iterator<Item = &'a Fragment>,
    ) -> Result<(), DbError> {
        todo!()
    }

    pub fn add_block<'a>(
        &mut self,
        _parent_id: &BlockId,
        _block_id: &BlockId,
        _chain_length: ChainLength,
        _block_date: BlockDate,
        _fragments: impl IntoIterator<Item = &'a Fragment>,
    ) -> Result<(), DbError> {
        todo!()
    }

    /// this sets `BlockId` as the tip, overriding the current one, BUT add_block will still
    /// change the tip anyway if the chain_length increases, this is mostly to simplify garbage
    /// collection during bootstrap.
    pub fn set_tip(&mut self, _id: &BlockId) -> Result<bool, DbError> {
        todo!()
    }

    pub fn commit(self) -> Result<(), DbError> {
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

        txn.commit()?;

        Ok(())
    }
}

impl Txn {
    pub fn get_last_stable_block(&self) -> ChainLength {
        let stability: Stability =
            unsafe { std::mem::transmute(self.txn.root(Root::Stability as usize)) };

        stability.last_stable_block
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
