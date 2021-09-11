use super::error::ExplorerError;
use byteorder::{BigEndian, LittleEndian};
use chain_core::property::{Fragment as _, Serialize};
use chain_impl_mockchain::{
    certificate::Certificate,
    config::ConfigParam,
    fragment::Fragment,
    header::HeaderId,
    transaction::{self, InputEnum, Witness},
    value::Value,
};
use sanakirja::{btree, direct_repr, Commit, RootDb, Storable, UnsizedStorable};
use std::convert::TryFrom;
use std::{convert::TryInto, mem::size_of, path::Path, sync::Arc};
use zerocopy::{
    byteorder::{U32, U64},
    AsBytes, FromBytes,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct TxnErr<E: std::error::Error + 'static>(pub E);

#[derive(Debug, Clone, PartialEq, Eq, AsBytes, FromBytes)]
#[repr(transparent)]
pub struct B32(U32<BigEndian>);

#[derive(Debug, Clone, PartialEq, Eq, AsBytes, FromBytes)]
#[repr(transparent)]
pub struct L32(U32<LittleEndian>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsBytes, FromBytes)]
#[repr(transparent)]
pub struct B64(U64<BigEndian>);

#[derive(Debug, Clone, PartialEq, Eq, AsBytes, FromBytes)]
#[repr(transparent)]
pub struct L64(U64<LittleEndian>);

impl L64 {
    pub fn new(n: u64) -> Self {
        Self(U64::<LittleEndian>::new(n))
    }

    pub fn get(&self) -> u64 {
        self.0.get()
    }
}

impl B64 {
    pub fn new(n: u64) -> Self {
        Self(U64::<BigEndian>::new(n))
    }

    pub fn get(&self) -> u64 {
        self.0.get()
    }
}

impl B32 {
    pub fn new(n: u32) -> Self {
        Self(U32::<BigEndian>::new(n))
    }

    pub fn get(&self) -> u32 {
        self.0.get()
    }
}

impl L32 {
    pub fn new(n: u32) -> Self {
        Self(U32::<LittleEndian>::new(n))
    }

    pub fn get(&self) -> u32 {
        self.0.get()
    }
}

impl AsRef<U64<LittleEndian>> for L64 {
    fn as_ref(&self) -> &U64<LittleEndian> {
        &self.0
    }
}

impl Ord for B64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.as_bytes().cmp(other.0.as_bytes())
    }
}

impl PartialOrd for B64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.as_bytes().partial_cmp(other.0.as_bytes())
    }
}

impl Ord for B32 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.as_bytes().cmp(&other.0.as_bytes())
    }
}

impl PartialOrd for B32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.as_bytes().partial_cmp(other.0.as_bytes())
    }
}

impl Ord for L64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.get().cmp(&other.0.get())
    }
}

impl PartialOrd for L64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.get().partial_cmp(&other.0.get())
    }
}

impl Ord for L32 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.get().cmp(&other.0.get())
    }
}

impl PartialOrd for L32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.get().partial_cmp(&other.0.get())
    }
}

direct_repr!(B32);
direct_repr!(L32);
direct_repr!(B64);
direct_repr!(L64);

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[repr(C)]
pub struct StakePoolMeta {
    pub registration: FragmentId,
    pub retirement: Option<FragmentId>,
}

direct_repr!(StakePoolMeta);

pub type SlotId = B32;
pub type EpochNumber = B32;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, AsBytes, FromBytes)]
#[repr(C)]
pub struct BlockDate {
    pub epoch: EpochNumber,
    pub slot_id: SlotId,
}

impl From<chain_impl_mockchain::block::BlockDate> for BlockDate {
    fn from(d: chain_impl_mockchain::block::BlockDate) -> Self {
        Self {
            epoch: B32::new(d.epoch),
            slot_id: B32::new(d.slot_id),
        }
    }
}

pub type ChainLength = B32;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
#[repr(C)]
pub struct BlockMeta {
    chain_length: ChainLength,
    date: BlockDate,
    parent_hash: BlockId,
}

direct_repr!(BlockMeta);

pub type PoolId = StorableHash;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct BlockProducer {
    bytes: [u8; 32],
}

direct_repr!(BlockProducer);

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, AsBytes)]
#[repr(C)]
pub struct VotePlanMeta {
    pub vote_start: BlockDate,
    pub vote_end: BlockDate,
    pub committee_end: BlockDate,
    pub payload_type: PayloadType,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, AsBytes)]
#[repr(u8)]
pub enum PayloadType {
    Public = 1,
    Private = 2,
}

impl From<chain_impl_mockchain::vote::PayloadType> for PayloadType {
    fn from(p: chain_impl_mockchain::vote::PayloadType) -> Self {
        match p {
            chain_impl_mockchain::vote::PayloadType::Public => PayloadType::Public,
            chain_impl_mockchain::vote::PayloadType::Private => PayloadType::Private,
        }
    }
}

direct_repr!(VotePlanMeta);

pub type ExternalProposalId = StorableHash;
pub type Options = u8;

#[derive(Debug, FromBytes, AsBytes, PartialEq, Eq, Ord, PartialOrd)]
#[repr(C)]
pub struct ExplorerVoteProposal {
    pub proposal_id: ExternalProposalId,
    pub options: Options,
    // pub tally: Option<ExplorerVoteTally>,
}

impl From<&chain_impl_mockchain::certificate::Proposal> for ExplorerVoteProposal {
    fn from(p: &chain_impl_mockchain::certificate::Proposal) -> Self {
        ExplorerVoteProposal {
            proposal_id: StorableHash::from(<[u8; 32]>::from(p.external_id().clone())),
            options: p.options().choice_range().end,
        }
    }
}

direct_repr!(ExplorerVoteProposal);

pub(crate) type P<K, V> = btree::page::Page<K, V>;
type Db<K, V> = btree::Db<K, V>;
pub(crate) type UP<K, V> = btree::page_unsized::Page<K, V>;
type UDb<K, V> = btree::Db_<K, V, UP<K, V>>;

#[derive(Debug, thiserror::Error)]
pub enum SanakirjaError {
    #[error(transparent)]
    Sanakirja(#[from] ::sanakirja::Error),
    #[error("Pristine locked")]
    PristineLocked,
    #[error("Pristine corrupt")]
    PristineCorrupt,
    #[error("version error")]
    Version,
}

impl std::convert::From<::sanakirja::CRCError> for SanakirjaError {
    fn from(_: ::sanakirja::CRCError) -> Self {
        SanakirjaError::PristineCorrupt
    }
}

impl std::convert::From<::sanakirja::CRCError> for TxnErr<SanakirjaError> {
    fn from(_: ::sanakirja::CRCError) -> Self {
        TxnErr(SanakirjaError::PristineCorrupt)
    }
}

impl std::convert::From<::sanakirja::Error> for TxnErr<SanakirjaError> {
    fn from(e: ::sanakirja::Error) -> Self {
        TxnErr(e.into())
    }
}

impl std::convert::From<TxnErr<::sanakirja::Error>> for TxnErr<SanakirjaError> {
    fn from(e: TxnErr<::sanakirja::Error>) -> Self {
        TxnErr(e.0.into())
    }
}

// A Sanakirja pristine.
#[derive(Clone)]
pub struct Pristine {
    pub env: Arc<::sanakirja::Env>,
}

impl Pristine {
    pub fn new<P: AsRef<Path>>(name: P) -> Result<Self, SanakirjaError> {
        Self::new_with_size(name, 1 << 20)
    }
    pub unsafe fn new_nolock<P: AsRef<Path>>(name: P) -> Result<Self, SanakirjaError> {
        Self::new_with_size_nolock(name, 1 << 20)
    }
    pub fn new_with_size<P: AsRef<Path>>(name: P, size: u64) -> Result<Self, SanakirjaError> {
        let env = ::sanakirja::Env::new(name, size, 2);
        match env {
            Ok(env) => Ok(Pristine { env: Arc::new(env) }),
            Err(::sanakirja::Error::IO(e)) => {
                if let std::io::ErrorKind::WouldBlock = e.kind() {
                    Err(SanakirjaError::PristineLocked)
                } else {
                    Err(SanakirjaError::Sanakirja(::sanakirja::Error::IO(e)))
                }
            }
            Err(e) => Err(SanakirjaError::Sanakirja(e)),
        }
    }
    pub unsafe fn new_with_size_nolock<P: AsRef<Path>>(
        name: P,
        size: u64,
    ) -> Result<Self, SanakirjaError> {
        Ok(Pristine {
            env: Arc::new(::sanakirja::Env::new_nolock(name, size, 2)?),
        })
    }
    pub fn new_anon() -> Result<Self, SanakirjaError> {
        Self::new_anon_with_size(1 << 20)
    }
    pub fn new_anon_with_size(size: u64) -> Result<Self, SanakirjaError> {
        Ok(Pristine {
            env: Arc::new(::sanakirja::Env::new_anon(size, 2)?),
        })
    }
}

#[derive(Debug, AsBytes, FromBytes)]
#[repr(C)]
pub struct Stability {
    epoch_stability_depth: L32,
    last_stable_block: ChainLength,
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

#[derive(Debug, AsBytes, FromBytes)]
#[repr(C)]
pub struct StaticSettings {
    discrimination: L32,
    consensus: L32,
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
    ChainLenghts,
    Tips,
    StakePoolData,
    States,
}

impl Pristine {
    pub fn txn_begin(&self) -> Result<Txn, SanakirjaError> {
        let txn = ::sanakirja::Env::txn_begin(self.env.clone())?;
        fn begin(txn: ::sanakirja::Txn<Arc<::sanakirja::Env>>) -> Option<Txn> {
            Some(Txn {
                states: txn.root_db(Root::States as usize)?,
                tips: txn.root_db(Root::Tips as usize)?,
                chain_lengths: txn.root_db(Root::ChainLenghts as usize)?,
                transaction_inputs: txn.root_db(Root::TransactionInputs as usize)?,
                transaction_outputs: txn.root_db(Root::TransactionOutputs as usize)?,
                transaction_certificates: txn.root_db(Root::TransactionCertificates as usize)?,
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
            Err(SanakirjaError::PristineCorrupt)
        }
    }

    pub fn mut_txn_begin(&self) -> Result<MutTxn<()>, SanakirjaError> {
        let mut txn = ::sanakirja::Env::mut_txn_begin(self.env.clone()).unwrap();
        Ok(MutTxn {
            states: if let Some(db) = txn.root_db(Root::States as usize) {
                db
            } else {
                btree::create_db_(&mut txn)?
            },
            tips: if let Some(db) = txn.root_db(Root::Tips as usize) {
                db
            } else {
                btree::create_db_(&mut txn)?
            },
            chain_lengths: if let Some(db) = txn.root_db(Root::ChainLenghts as usize) {
                db
            } else {
                btree::create_db_(&mut txn)?
            },
            transaction_inputs: if let Some(db) = txn.root_db(Root::TransactionInputs as usize) {
                db
            } else {
                btree::create_db_(&mut txn)?
            },
            transaction_outputs: if let Some(db) = txn.root_db(Root::TransactionOutputs as usize) {
                db
            } else {
                btree::create_db_(&mut txn)?
            },
            transaction_certificates: if let Some(db) =
                txn.root_db(Root::TransactionCertificates as usize)
            {
                db
            } else {
                btree::create_db_(&mut txn)?
            },
            blocks: if let Some(db) = txn.root_db(Root::Blocks as usize) {
                db
            } else {
                btree::create_db_(&mut txn)?
            },
            block_transactions: if let Some(db) = txn.root_db(Root::BlockTransactions as usize) {
                db
            } else {
                btree::create_db_(&mut txn)?
            },
            vote_plans: if let Some(db) = txn.root_db(Root::VotePlans as usize) {
                db
            } else {
                btree::create_db_(&mut txn)?
            },
            vote_plan_proposals: if let Some(db) = txn.root_db(Root::VotePlanProposals as usize) {
                db
            } else {
                btree::create_db_(&mut txn)?
            },
            stake_pool_data: if let Some(db) = txn.root_db(Root::StakePoolData as usize) {
                db
            } else {
                btree::create_db_(&mut txn)?
            },
            txn,
        })
    }
}

pub type Txn = GenericTxn<::sanakirja::Txn<Arc<::sanakirja::Env>>>;
pub type MutTxn<T> = GenericTxn<::sanakirja::MutTxn<Arc<::sanakirja::Env>, T>>;

// pub type Transactions = UDb<Pair<FragmentId, TxComponentTag>, TxComponent>;
// pub type TransactionCursor = btree::Cursor<
//     Pair<FragmentId, TxComponentTag>,
//     TxComponent,
//     btree::page_unsized::Page<Pair<FragmentId, TxComponentTag>, TxComponent>,
// >;

pub type TransactionsInputs = Db<Pair<FragmentId, u8>, TransactionInput>;
pub type TransactionInputsCursor = btree::Cursor<
    Pair<FragmentId, u8>,
    TransactionInput,
    P<Pair<FragmentId, u8>, TransactionInput>,
>;

pub type TransactionsOutputs = Db<Pair<FragmentId, u8>, TransactionOutput>;
pub type TransactionOutputsCursor = btree::Cursor<
    Pair<FragmentId, u8>,
    TransactionOutput,
    P<Pair<FragmentId, u8>, TransactionOutput>,
>;

pub type TransactionsCertificate = UDb<FragmentId, TransactionCertificate>;
pub type TransactionsCertificateCursor = btree::Cursor<
    FragmentId,
    TransactionCertificate,
    btree::page_unsized::Page<FragmentId, TransactionCertificate>,
>;

const fn max(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes, FromBytes)]
#[repr(C)]
pub struct TransactionInput {
    pub input_ptr: [u8; 32],
    pub value: L64,
    pub utxo_or_account: u8,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum InputType {
    Utxo = 0x00,
    Account = 0xff,
}

impl From<&transaction::Input> for TransactionInput {
    fn from(i: &transaction::Input) -> Self {
        TransactionInput {
            input_ptr: i.bytes()[9..].try_into().unwrap(),
            utxo_or_account: match i.get_type() {
                transaction::InputType::Utxo => InputType::Utxo as u8,
                transaction::InputType::Account => InputType::Account as u8,
            },
            value: L64::new(i.value().0),
        }
    }
}

impl From<&TransactionInput> for transaction::Input {
    fn from(input: &TransactionInput) -> Self {
        transaction::Input::new(
            input.utxo_or_account,
            Value(input.value.get()),
            input.input_ptr,
        )
    }
}

direct_repr!(TransactionInput);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, AsBytes, FromBytes)]
#[repr(C)]
pub struct TransactionOutput {
    pub address: Address,
    pub value: L64,
}

impl TransactionOutput {
    pub fn from_original(output: &transaction::Output<chain_addr::Address>) -> Self {
        TransactionOutput {
            address: Address::from(output.address.clone()),
            value: L64::new(output.value.0),
        }
    }
}

impl From<&TransactionOutput> for transaction::Output<chain_addr::Address> {
    fn from(output: &TransactionOutput) -> Self {
        transaction::Output {
            address: output.address.clone().try_into().unwrap(),
            value: Value(output.value.get()),
        }
    }
}

direct_repr!(TransactionOutput);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(C)]
pub struct TransactionCertificate {
    tag: CertificateTag,
    cert: SerializedCertificate,
}

impl TransactionCertificate {
    const fn alloc() -> [u8; size_of::<SerializedCertificate>()] {
        [0u8; size_of::<SerializedCertificate>()]
    }

    pub fn from_vote_plan_meta(meta: VotePlanMeta) -> Self {
        let mut alloc = [0u8; size_of::<SerializedCertificate>()];
        alloc.copy_from_slice(meta.as_bytes());

        TransactionCertificate {
            tag: CertificateTag::VotePlan,
            cert: SerializedCertificate(alloc),
        }
    }

    pub fn from_public_vote_cast(vote: PublicVoteCast) -> Self {
        let mut alloc = Self::alloc();
        alloc.copy_from_slice(vote.as_bytes());

        TransactionCertificate {
            tag: CertificateTag::VoteCast,
            cert: SerializedCertificate(alloc),
        }
    }
}

direct_repr!(TransactionCertificate);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(u8)]
enum CertificateTag {
    VotePlan = 0,
    VoteCast = 1,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(C)]
pub struct SerializedCertificate(
    [u8; max(
        std::mem::size_of::<VotePlanMeta>(),
        std::mem::size_of::<PublicVoteCast>(),
    )],
);

pub type Choice = u8;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(C)]
pub struct PublicVoteCast {
    vote_plan: VotePlanId,
    proposal_index: u8,
    payload: Choice,
}

pub type Blocks = Db<BlockId, BlockMeta>;
pub type BlockTransactions = Db<BlockId, Pair<u8, FragmentId>>;
pub type BlockTransactionsCursor =
    btree::Cursor<BlockId, Pair<u8, FragmentId>, P<BlockId, Pair<u8, FragmentId>>>;
pub type ChainLengths = Db<ChainLength, BlockId>;
pub type ChainLengthsCursor = btree::Cursor<ChainLength, BlockId, P<ChainLength, BlockId>>;
pub type VotePlans = UDb<VotePlanId, VotePlanMeta>;
pub type VotePlanProposals = UDb<VotePlanId, Pair<u8, ExplorerVoteProposal>>;
pub type StakePools = UDb<PoolId, StakePoolMeta>;
pub type Tips = Db<Pair<B32, BlockId>, ()>;
pub type TipsCursor = btree::Cursor<Pair<B32, BlockId>, (), P<Pair<B32, BlockId>, ()>>;

// multiverse
pub type States = Db<BlockId, SerializedStateRef>;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct AccountId([u8; chain_impl_mockchain::transaction::INPUT_PTR_SIZE]);
direct_repr!(AccountId);

impl std::fmt::Debug for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

pub type ProposalIndex = u8;
pub type ProposalId = Pair<VotePlanId, ProposalIndex>;
pub type StakeControl = Db<AccountId, Stake>;
pub type StakePoolBlocks = Db<PoolIdRecord, BlockId>;
pub type BlocksInBranch = Db<ChainLength, BlockId>;

pub type AddressId = SeqNum;
pub type AddressIds = Db<Address, AddressId>;
pub type AddressTransactions = Db<AddressId, Pair<SeqNum, FragmentId>>;
pub type AddressTransactionsCursor =
    btree::Cursor<AddressId, Pair<SeqNum, FragmentId>, P<AddressId, Pair<SeqNum, FragmentId>>>;
pub type Votes = Db<ProposalId, Pair<SeqNum, Choice>>;

// a typed (and in-memory) version of SerializedStateRef
pub struct StateRef {
    stake_pool_blocks: StakePoolBlocks,
    stake_control: StakeControl,
    blocks: BlocksInBranch,
    address_id: AddressIds,
    address_transactions: AddressTransactions,
    votes: Votes,
    next_address_id: Option<SeqNum>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct SerializedStateRef {
    pub stake_pool_blocks: L64,
    pub stake_control: L64,
    pub blocks: L64,
    pub address_id: L64,
    pub addresses: L64,
    pub votes: L64,
}

impl From<SerializedStateRef> for StateRef {
    fn from(ser: SerializedStateRef) -> Self {
        StateRef {
            stake_pool_blocks: Db::from_page(ser.stake_pool_blocks.get()),
            stake_control: Db::from_page(ser.stake_control.get()),
            blocks: Db::from_page(ser.blocks.get()),
            address_id: Db::from_page(ser.address_id.get()),
            address_transactions: Db::from_page(ser.addresses.get()),
            votes: Db::from_page(ser.votes.get()),
            next_address_id: None,
        }
    }
}

type BlockId = StorableHash;

impl From<BlockId> for HeaderId {
    fn from(val: BlockId) -> Self {
        HeaderId::from(val.0)
    }
}

type FragmentId = StorableHash;
type VotePlanId = StorableHash;

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, AsBytes, FromBytes)]
#[cfg_attr(test, derive(Hash))]
#[repr(C)]
pub struct StorableHash(pub [u8; 32]);

direct_repr!(StorableHash);

impl StorableHash {
    const MIN: Self = StorableHash([0x00; 32]);
    const MAX: Self = StorableHash([0xff; 32]);
}

impl From<chain_impl_mockchain::key::Hash> for StorableHash {
    fn from(id: chain_impl_mockchain::key::Hash) -> Self {
        let bytes: [u8; 32] = id.into();

        Self(bytes)
    }
}

impl From<[u8; 32]> for StorableHash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl std::fmt::Debug for StorableHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

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
    pub blocks: Blocks,
    pub block_transactions: BlockTransactions,
    pub vote_plans: VotePlans,
    pub vote_plan_proposals: VotePlanProposals,
    pub stake_pool_data: StakePools,
}

impl<T: ::sanakirja::LoadPage<Error = ::sanakirja::Error> + ::sanakirja::RootPage> GenericTxn<T> {}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[repr(C)]
pub struct Pair<A, B> {
    pub a: A,
    pub b: B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct SeqNum(B64);

direct_repr!(SeqNum);

impl SeqNum {
    pub const MAX: SeqNum = SeqNum(B64(U64::<BigEndian>::MAX_VALUE));
    pub const MIN: SeqNum = SeqNum(B64(U64::<BigEndian>::ZERO));

    pub fn new(n: u64) -> Self {
        Self(B64::new(n))
    }

    pub fn next(self) -> SeqNum {
        Self::new(self.0.get() + 1)
    }
}

pub type Stake = L64;
pub type PoolIdRecord = Pair<PoolId, SeqNum>;

const MAX_ADDRESS_SIZE: usize = chain_addr::ADDR_SIZE_GROUP;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, AsBytes, FromBytes)]
#[repr(C)]
pub struct Address([u8; MAX_ADDRESS_SIZE]);

impl Address {
    const MIN: Address = Address([0u8; MAX_ADDRESS_SIZE]);
    const MAX: Address = Address([255u8; MAX_ADDRESS_SIZE]);
}

direct_repr!(Address);

impl From<chain_addr::Address> for Address {
    fn from(addr: chain_addr::Address) -> Self {
        let mut bytes = [0u8; MAX_ADDRESS_SIZE];
        addr.serialize(&mut bytes[..]).unwrap();
        Self(bytes)
    }
}

impl From<&chain_addr::Address> for Address {
    fn from(addr: &chain_addr::Address) -> Self {
        let mut bytes = [0u8; MAX_ADDRESS_SIZE];
        addr.serialize(&mut bytes[..]).unwrap();
        Self(bytes)
    }
}

impl TryInto<chain_addr::Address> for Address {
    type Error = chain_addr::Error;

    fn try_into(self) -> Result<chain_addr::Address, Self::Error> {
        chain_addr::Address::from_bytes(&self.0[0..33])
            .or_else(|_| chain_addr::Address::from_bytes(&self.0[0..MAX_ADDRESS_SIZE]))
    }
}

impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let addr: chain_addr::Address = self.clone().try_into().unwrap();
        // addr.fmt(f)
        //
        f.write_str(&hex::encode(self.0))
    }
}

impl StateRef {
    pub fn new_empty<T>(txn: &mut T) -> Self
    where
        T: ::sanakirja::AllocPage
            + ::sanakirja::LoadPage<Error = ::sanakirja::Error>
            + ::sanakirja::RootPage,
    {
        let mut s = Self {
            stake_pool_blocks: btree::create_db_(txn).unwrap(),
            stake_control: btree::create_db_(txn).unwrap(),
            blocks: btree::create_db_(txn).unwrap(),
            address_id: btree::create_db_(txn).unwrap(),
            address_transactions: btree::create_db_(txn).unwrap(),
            votes: btree::create_db_(txn).unwrap(),

            next_address_id: None,
        };

        // TODO: extract [0u8; 65] to an Address::sigil function
        btree::put(txn, &mut s.address_id, &Address([0u8; 65]), &SeqNum::new(0)).unwrap();

        s
    }

    pub fn finish(mut self, txn: &mut SanakirjaMutTx) -> SerializedStateRef {
        // if the sequence counter for addresses was incremented previously, rewrite it
        if let Some(next_seq) = self.next_address_id {
            btree::del(txn, &mut self.address_id, &Address([0u8; 65]), None).unwrap();

            debug_assert!(btree::put(
                txn,
                &mut self.address_id,
                &Address([0u8; 65]),
                &next_seq.next(),
            )
            .unwrap());
        }

        SerializedStateRef {
            stake_pool_blocks: L64::new(self.stake_pool_blocks.db),
            stake_control: L64::new(self.stake_control.db),
            blocks: L64::new(self.blocks.db),
            address_id: L64::new(self.address_id.db),
            addresses: L64::new(self.address_transactions.db),
            votes: L64::new(self.votes.db),
        }
    }

    pub fn apply_vote(
        &mut self,
        txn: &mut SanakirjaMutTx,
        vote_plan_id: &VotePlanId,
        proposal_index: u8,
        choice: Choice,
    ) -> Result<(), ExplorerError> {
        let proposal_id = Pair {
            a: vote_plan_id.clone(),
            b: proposal_index,
        };

        let max_possible_value = Pair {
            a: SeqNum::MAX,
            b: u8::MAX,
        };

        let seq = find_last_record_by(txn, &self.votes, &proposal_id, &max_possible_value)
            .map(|last| last.a.next())
            .unwrap_or(SeqNum::MIN);

        btree::put(
            txn,
            &mut self.votes,
            &proposal_id,
            &Pair { a: seq, b: choice },
        )
        .unwrap();

        Ok(())
    }

    /// Add the given transaction to address at the end of the list
    /// This function *only* checks the last fragment to avoid repetition when a transaction has more
    /// than one (input|output) with the same address (eg: utxo input and change).
    pub fn add_transaction_to_address(
        &mut self,
        txn: &mut SanakirjaMutTx,
        fragment_id: &FragmentId,
        address: &Address,
    ) -> Result<(), ExplorerError> {
        let address_id = self.get_or_insert_address_id(txn, address);

        let max_possible_value = Pair {
            a: SeqNum::MAX,
            b: FragmentId::MAX,
        };

        let seq = match find_last_record_by(
            &*txn,
            &self.address_transactions,
            &address_id,
            &max_possible_value,
        ) {
            Some(v) => {
                if &v.b == fragment_id {
                    return Ok(());
                } else {
                    v.a.next()
                }
            }
            None => SeqNum::MIN,
        };

        debug_assert!(btree::put(
            txn,
            &mut self.address_transactions,
            &address_id,
            &Pair {
                a: seq,
                b: fragment_id.clone(),
            },
        )
        .unwrap());

        Ok(())
    }

    pub fn add_block_to_blocks(
        &mut self,
        txn: &mut SanakirjaMutTx,
        chain_length: &ChainLength,
        block_id: &BlockId,
    ) -> Result<(), ExplorerError> {
        btree::put(txn, &mut self.blocks, chain_length, block_id).unwrap();
        Ok(())
    }

    pub(crate) fn get_or_insert_address_id(
        &mut self,
        txn: &mut SanakirjaMutTx,
        address: &Address,
    ) -> SeqNum {
        let address_exists = btree::get(txn, &self.address_id, address, None)
            .unwrap()
            .filter(|(id, _)| id == &address)
            .map(|(_, v)| v)
            .cloned();

        let address_id = if let Some(v) = address_exists {
            v
        } else {
            let next_seq = if let Some(next_seq) = self.next_address_id {
                next_seq
            } else {
                *btree::get(txn, &self.address_id, &Address([0u8; 65]), None)
                    .unwrap()
                    .unwrap()
                    .1
            };

            self.next_address_id = Some(next_seq.next());

            btree::put(txn, &mut self.address_id, address, &next_seq).unwrap();

            next_seq
        };

        address_id
    }

    pub fn apply_output_to_stake_control(
        &mut self,
        txn: &mut SanakirjaMutTx,
        output: &transaction::Output<chain_addr::Address>,
    ) -> Result<(), ExplorerError> {
        match output.address.kind() {
            chain_addr::Kind::Group(_, account) => {
                self.add_stake_to_account(txn, account, output.value);
            }
            chain_addr::Kind::Account(account) => {
                self.add_stake_to_account(txn, account, output.value);
            }
            chain_addr::Kind::Single(_account) => {}
            chain_addr::Kind::Multisig(_) => {}
            chain_addr::Kind::Script(_) => {}
        }
        Ok(())
    }

    fn add_stake_to_account(
        &mut self,
        txn: &mut SanakirjaMutTx,
        account: &chain_crypto::PublicKey<chain_crypto::Ed25519>,
        value: Value,
    ) {
        dbg!("adding transaction to account");
        let op = |current_stake: u64, value: u64| -> u64 {
            dbg!(current_stake).checked_add(dbg!(value)).unwrap()
        };

        self.update_stake_for_account(txn, account, op, value);
    }

    fn substract_stake_from_account(
        &mut self,
        txn: &mut SanakirjaMutTx,
        account: &chain_crypto::PublicKey<chain_crypto::Ed25519>,
        value: Value,
    ) {
        dbg!("adding transaction to account");
        let op = |current_stake: u64, value: u64| -> u64 {
            dbg!(current_stake).checked_sub(dbg!(value)).unwrap()
        };

        self.update_stake_for_account(txn, account, op, value);
    }

    fn update_stake_for_account(
        &mut self,
        txn: &mut SanakirjaMutTx,
        account: &chain_crypto::PublicKey<chain_crypto::Ed25519>,
        op: impl Fn(u64, u64) -> u64,
        value: Value,
    ) {
        let account_id = AccountId(account.as_ref().try_into().unwrap());

        let current_stake = btree::get(txn, &self.stake_control, &account_id, None)
            .unwrap()
            .and_then(|(k, stake)| {
                if dbg!(k) == dbg!(&account_id) {
                    Some(stake.get())
                } else {
                    None
                }
            })
            .unwrap_or(0);

        let new_stake = dbg!(op(current_stake, value.0));

        btree::del(txn, &mut self.stake_control, &account_id, None).unwrap();
        btree::put(
            txn,
            &mut self.stake_control,
            &account_id,
            &L64::new(new_stake),
        )
        .unwrap();
    }
}

type SanakirjaMutTx = ::sanakirja::MutTxn<Arc<::sanakirja::Env>, ()>;
type SanakirjaTx = ::sanakirja::Txn<Arc<::sanakirja::Env>>;

impl SerializedStateRef {
    pub fn fork(&self, txn: &mut SanakirjaMutTx) -> StateRef {
        StateRef {
            stake_pool_blocks: btree::fork_db(txn, &Db::from_page(self.stake_pool_blocks.get()))
                .unwrap(),
            stake_control: btree::fork_db(txn, &Db::from_page(self.stake_control.get())).unwrap(),
            blocks: btree::fork_db(txn, &Db::from_page(self.blocks.get())).unwrap(),
            address_id: btree::fork_db(txn, &Db::from_page(self.address_id.get())).unwrap(),
            address_transactions: btree::fork_db(txn, &Db::from_page(self.addresses.get()))
                .unwrap(),
            votes: btree::fork_db(txn, &Db::from_page(self.votes.get())).unwrap(),
            next_address_id: None,
        }
    }
}

direct_repr!(SerializedStateRef);

impl MutTxn<()> {
    pub fn add_block0(
        &mut self,
        parent_id: &BlockId,
        block0_id: &BlockId,
        fragments: impl Iterator<Item = Fragment>,
    ) -> Result<(), ExplorerError> {
        let state_ref = StateRef::new_empty(&mut self.txn);

        unsafe {
            self.txn.set_root(
                Root::Stability as usize,
                std::mem::transmute(Stability::default()),
            )
        };

        let tip = Pair {
            a: B32::new(0),
            b: block0_id.clone(),
        };

        assert!(btree::put(&mut self.txn, &mut self.tips, &tip, &()).unwrap());

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

    pub fn add_block(
        &mut self,
        parent_id: &BlockId,
        block_id: &BlockId,
        chain_length: ChainLength,
        block_date: BlockDate,
        fragments: impl IntoIterator<Item = Fragment>,
    ) -> Result<(), ExplorerError> {
        self.update_tips(&parent_id, chain_length.clone(), &block_id)?;

        let states = btree::get(&self.txn, &self.states, &parent_id, None)
            .unwrap()
            .filter(|(branch_id, _states)| *branch_id == parent_id)
            .map(|(_branch_id, states)| states)
            .cloned()
            .ok_or_else(|| ExplorerError::AncestorNotFound(block_id.clone().into()))?;

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

    fn update_state(
        &mut self,
        fragments: impl Iterator<Item = Fragment>,
        mut state_ref: StateRef,
        chain_length: ChainLength,
        block_id: &StorableHash,
        block_date: BlockDate,
        parent_id: &StorableHash,
    ) -> Result<(), ExplorerError> {
        dbg!(format!(
            "------------------- adding state for {:?} -------------",
            &block_id
        ));

        self.apply_fragments(&block_id, fragments, &mut state_ref)?;
        state_ref.add_block_to_blocks(&mut self.txn, &chain_length, &block_id)?;

        let new_state = state_ref.finish(&mut self.txn);

        if !btree::put(&mut self.txn, &mut self.states, &block_id, &new_state).unwrap() {
            return Err(ExplorerError::BlockAlreadyExists(block_id.clone().into()));
        }

        self.update_chain_lengths(chain_length.clone(), &block_id)?;

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

    fn apply_fragments(
        &mut self,
        block_id: &BlockId,
        fragments: impl Iterator<Item = Fragment>,
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
                    b: dbg!(fragment_id.clone()),
                },
            )
            .unwrap();

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
                    self.apply_transaction(fragment_id, &tx, state_ref)?;
                    self.add_vote_plan_meta(tx);
                }
                Fragment::VoteCast(tx) => {
                    self.apply_transaction(fragment_id.clone(), &tx, state_ref)?;

                    let vote_cast = tx.as_slice().payload().into_payload();
                    let vote_plan_id =
                        StorableHash(<[u8; 32]>::from(vote_cast.vote_plan().clone()));

                    let proposal_index = vote_cast.proposal_index();
                    match vote_cast.payload() {
                        chain_impl_mockchain::vote::Payload::Public { choice } => {
                            state_ref.apply_vote(
                                &mut self.txn,
                                &vote_plan_id,
                                proposal_index,
                                choice.as_byte(),
                            )?;
                        }
                        // private vote not supported yet
                        chain_impl_mockchain::vote::Payload::Private {
                            encrypted_vote: _,
                            proof: _,
                        } => (),
                    }
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

    fn add_vote_plan_meta(
        &mut self,
        tx: &transaction::Transaction<chain_impl_mockchain::certificate::VotePlan>,
    ) {
        let vote_plan = tx.as_slice().payload().into_payload();
        let vote_plan_id = StorableHash(<[u8; 32]>::from(vote_plan.to_id()));
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
                &vote_plan_id,
                &Pair {
                    a: idx as u8,
                    b: proposal.into(),
                },
            )
            .unwrap();
        }

        btree::put(
            &mut self.txn,
            &mut self.vote_plans,
            &vote_plan_id,
            &vote_plan_meta,
        )
        .unwrap();
    }

    fn apply_transaction<P>(
        &mut self,
        fragment_id: FragmentId,
        tx: &transaction::Transaction<P>,
        state: &mut StateRef,
    ) -> Result<(), ExplorerError> {
        dbg!(tx);
        let etx = tx.as_slice();

        // is important that we put outputs first, because utxo's can refer to inputs in the same
        // transaction, so those need to be already indexed. Although it would be technically
        // faster to just look for them in the serialized transaction, that's increases complexity
        // for something that is not really that likely. Besides, the pages should be in the system
        // cache because we recently inserted them.
        for (idx, output) in etx.outputs().iter().enumerate() {
            self.put_transaction_output(fragment_id.clone(), idx as u8, &output)
                .unwrap();
            state.apply_output_to_stake_control(&mut self.txn, &output)?;
            state.add_transaction_to_address(
                &mut self.txn,
                &fragment_id,
                &output.address.into(),
            )?;
        }

        for (index, (input, witness)) in etx.inputs_and_witnesses().iter().enumerate() {
            self.put_transaction_input(fragment_id.clone(), index as u8, &input)
                .unwrap();

            let resolved_utxo = match input.to_enum() {
                InputEnum::AccountInput(_, _) => None,
                InputEnum::UtxoInput(input) => {
                    Some(self.resolve_utxo(&self.transaction_outputs, input).clone())
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

    pub fn update_tips(
        &mut self,
        parent_id: &BlockId,
        chain_length: ChainLength,
        block_id: &BlockId,
    ) -> Result<(), ExplorerError> {
        let parent_key = Pair {
            a: B32::new(
                chain_length
                    .get()
                    .checked_sub(1)
                    .expect("update tips called with block0"),
            ),
            b: parent_id.clone(),
        };

        let _ = btree::del(&mut self.txn, &mut self.tips, &parent_key, None).unwrap();

        let key = Pair {
            a: B32::new(chain_length.get()),
            b: block_id.clone(),
        };

        btree::put(&mut self.txn, &mut self.tips, &key, &()).unwrap();

        Ok(())
    }

    pub fn update_chain_lengths(
        &mut self,
        chain_length: ChainLength,
        block_id: &BlockId,
    ) -> Result<(), ExplorerError> {
        btree::put(
            &mut self.txn,
            &mut self.chain_lengths,
            &chain_length,
            block_id,
        )
        .unwrap();

        Ok(())
    }

    pub fn put_transaction_input(
        &mut self,
        fragment_id: FragmentId,
        index: u8,
        input: &transaction::Input,
    ) -> Result<(), ExplorerError> {
        btree::put(
            &mut self.txn,
            &mut self.transaction_inputs,
            &Pair {
                a: fragment_id,
                b: index,
            },
            &TransactionInput::from(input),
        )
        .unwrap();

        Ok(())
    }

    pub fn put_transaction_output(
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
        )
        .unwrap();

        Ok(())
    }

    pub fn put_transaction_certificate(
        &mut self,
        fragment_id: FragmentId,
        cert: TransactionCertificate,
    ) -> Result<(), ExplorerError> {
        btree::put(
            &mut self.txn,
            &mut self.transaction_certificates,
            &fragment_id,
            &cert,
        )
        .unwrap();

        Ok(())
    }

    pub fn update_stake_pool_meta(&mut self, fragment: &Fragment) -> Result<(), ExplorerError> {
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
                    &StorableHash(<[u8; 32]>::from(stake_pool_id)),
                    &StakePoolMeta {
                        registration: StorableHash::from(fragment.id()),
                        retirement: None,
                    },
                )
                .unwrap();
            }
            Fragment::PoolRetirement(tx) => {
                let etx = tx.as_slice();
                let cert = etx.payload();

                let stake_pool_id = match cert.into_certificate_slice().unwrap().into_owned() {
                    Certificate::PoolRetirement(r) => r.pool_id,
                    _ => unreachable!("mismatched certificate type"),
                };

                let stake_pool_id = StorableHash(<[u8; 32]>::from(stake_pool_id));

                let mut new = btree::get(&self.txn, &self.stake_pool_data, &stake_pool_id, None)
                    .unwrap()
                    .map(|(_, meta)| meta)
                    .cloned()
                    .unwrap();

                new.retirement = Some(FragmentId::from(fragment.id()));

                btree::del(
                    &mut self.txn,
                    &mut self.stake_pool_data,
                    &stake_pool_id,
                    None,
                )
                .unwrap();

                btree::put(
                    &mut self.txn,
                    &mut self.stake_pool_data,
                    &stake_pool_id,
                    &new,
                )
                .unwrap();
            }
            _ => {}
        }

        Ok(())
    }

    pub fn add_block_meta(
        &mut self,
        block_id: &BlockId,
        block: BlockMeta,
    ) -> Result<(), ExplorerError> {
        btree::put(&mut self.txn, &mut self.blocks, block_id, &block).unwrap();

        Ok(())
    }

    pub fn apply_input_to_stake_control(
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

    pub fn apply_input_to_transactions_by_address(
        &mut self,
        fragment_id: &FragmentId,
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
    ) -> &TransactionOutput {
        let txid = utxo_pointer.transaction_id;
        let idx = utxo_pointer.output_index;

        let mut cursor = btree::Cursor::new(&self.txn, &transactions).unwrap();

        let key = Pair {
            a: StorableHash::from(txid),
            b: idx,
        };

        cursor
            .set(
                &self.txn,
                &key,
                Some(&TransactionOutput {
                    // address: Address::MAX,
                    address: Address::MIN,
                    value: L64::new(u64::MIN),
                }),
            )
            .unwrap();

        if let Some((_, output)) = cursor
            .current(&self.txn)
            .unwrap()
            .filter(|(k, _)| *k == &key)
        {
            output
        } else {
            panic!("missing utxo {:?}", txid)
        }
    }

    pub fn commit(self) {
        // destructure things so we get some sort of exhaustiveness-check
        let Self {
            mut txn,
            states,
            tips,
            chain_lengths,
            transaction_inputs,
            transaction_outputs,
            transaction_certificates,
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
        txn.set_root(Root::Blocks as usize, blocks.db);
        txn.set_root(Root::BlockTransactions as usize, block_transactions.db);
        txn.set_root(Root::VotePlans as usize, vote_plans.db);
        txn.set_root(Root::VotePlanProposals as usize, vote_plan_proposals.db);
        txn.set_root(Root::StakePoolData as usize, stake_pool_data.db);

        txn.commit().unwrap();
    }
}

impl Txn {
    pub fn get_transactions_by_address<'a>(
        &'a self,
        state_id: &StorableHash,
        address: &Address,
        cursor: Option<SeqNum>,
    ) -> Result<Option<TxsByAddress<'a>>, ExplorerError> {
        let state = btree::get(&self.txn, &self.states, &state_id, None).unwrap();

        let state = match state {
            Some((s, state)) if state_id == s => StateRef::from(state.clone()),
            _ => return Ok(None),
        };

        let address_id = match btree::get(&self.txn, &state.address_id, &address, None).unwrap() {
            Some((a, id)) if a == address => id,
            _ => return Ok(None),
        };

        let max_possible_value = Pair {
            a: cursor.unwrap_or(SeqNum::MAX),
            b: FragmentId::MAX,
        };

        let mut cursor = btree::Cursor::new(&self.txn, &state.address_transactions).unwrap();

        btree::get(
            &self.txn,
            &state.address_transactions,
            &address_id,
            Some(&Pair {
                a: SeqNum::MAX,
                b: FragmentId::MAX,
            }),
        )
        .unwrap();

        cursor
            .set(&self.txn, &address_id, Some(&max_possible_value))
            .unwrap();

        if let Some((k, _)) = cursor.prev(&self.txn).unwrap() {
            if k == address_id {
                cursor.next(&self.txn).unwrap();
            }
        }

        Ok(Some(TxsByAddress {
            txn: &self.txn,
            address_id: *address_id,
            cursor,
        }))
    }

    pub fn get_branches(&self) -> BranchIter {
        let cursor = btree::Cursor::new(&self.txn, &self.tips).unwrap();
        BranchIter {
            txn: &self.txn,
            cursor,
        }
    }

    pub fn get_block_fragments(&self, block_id: &BlockId) -> BlockFragmentIter {
        let mut cursor = btree::Cursor::new(&self.txn, &self.block_transactions).unwrap();
        cursor.set(&self.txn, block_id, None).unwrap();

        BlockFragmentIter {
            txn: &self.txn,
            block_id: block_id.clone(),
            cursor,
        }
    }

    pub fn get_fragment_inputs(
        &self,
        fragment_id: &FragmentId,
        from: Option<u8>,
    ) -> impl Iterator<Item = &TransactionInput> {
        let mut cursor = btree::Cursor::new(&self.txn, &self.transaction_inputs).unwrap();
        let key = Pair {
            a: fragment_id.clone(),
            b: from.unwrap_or(0),
        };

        cursor.set(&self.txn, &key, None).unwrap();

        let iter = FragmentInputIter {
            txn: &self.txn,
            key,
            cursor,
        };

        iter
    }

    pub fn get_fragment_outputs(
        &self,
        fragment_id: &FragmentId,
        from: Option<u8>,
    ) -> impl Iterator<Item = &TransactionOutput> {
        let mut cursor = btree::Cursor::new(&self.txn, &self.transaction_outputs).unwrap();
        let key = Pair {
            a: fragment_id.clone(),
            b: from.unwrap_or(0),
        };

        cursor.set(&self.txn, &key, None).unwrap();

        let iter = FragmentOutputIter {
            txn: &self.txn,
            key,
            cursor,
        };

        iter
    }

    pub fn get_fragment_certificate(
        &self,
        fragment_id: &FragmentId,
    ) -> Option<&TransactionCertificate> {
        let key = fragment_id.clone();

        let certificate =
            btree::get(&self.txn, &self.transaction_certificates, &key, None).unwrap();

        certificate.and_then(|(k, v)| if k == &key { unsafe { Some(v) } } else { None })
    }

    pub fn get_blocks_by_chain_length(&self, chain_length: &ChainLength) -> ChainLengthIter {
        let mut cursor = btree::Cursor::new(&self.txn, &self.chain_lengths).unwrap();

        cursor.set(&self.txn, &chain_length, None).unwrap();

        ChainLengthIter {
            txn: &self.txn,
            key: chain_length.clone(),
            cursor,
        }
    }

    pub fn get_block_meta(&self, block_id: &BlockId) -> Option<&BlockMeta> {
        let certificate = btree::get(&self.txn, &self.blocks, &block_id, None).unwrap();

        certificate.and_then(|(k, v)| if k == block_id { Some(v) } else { None })
    }
}

pub struct TxsByAddress<'a> {
    txn: &'a SanakirjaTx,
    address_id: AddressId,
    cursor: AddressTransactionsCursor,
}

impl<'a> Iterator for TxsByAddress<'a> {
    type Item = FragmentId;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.prev(self.txn).unwrap().and_then(|(k, v)| {
            if k == &self.address_id {
                Some(v.b.clone())
            } else {
                None
            }
        })
    }

    // TODO: can probably implement size_hint, last, first...
}

pub struct BranchIter<'a> {
    txn: &'a SanakirjaTx,
    cursor: TipsCursor,
}

impl<'a> Iterator for BranchIter<'a> {
    type Item = FragmentId;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor
            .next(self.txn)
            .unwrap()
            .map(|(k, _)| k.b.clone())
    }
}

pub struct BlockFragmentIter<'a> {
    txn: &'a SanakirjaTx,
    block_id: BlockId,
    cursor: BlockTransactionsCursor,
}

impl<'a> Iterator for BlockFragmentIter<'a> {
    type Item = FragmentId;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next(self.txn).unwrap().and_then(|(k, v)| {
            if k == &self.block_id {
                Some(v.b.clone())
            } else {
                None
            }
        })
    }
}

pub struct FragmentInputIter<'a> {
    txn: &'a SanakirjaTx,
    key: Pair<FragmentId, u8>,
    cursor: TransactionInputsCursor,
}

impl<'a> Iterator for FragmentInputIter<'a> {
    type Item = &'a TransactionInput;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next(self.txn).unwrap().and_then(
            |(k, v)| {
                if k == &self.key {
                    Some(v)
                } else {
                    None
                }
            },
        )
    }
}

pub struct FragmentOutputIter<'a> {
    txn: &'a SanakirjaTx,
    key: Pair<FragmentId, u8>,
    cursor: TransactionOutputsCursor,
}

impl<'a> Iterator for FragmentOutputIter<'a> {
    type Item = &'a TransactionOutput;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next(self.txn).unwrap().and_then(
            |(k, v)| {
                if k == &self.key {
                    Some(v)
                } else {
                    None
                }
            },
        )
    }
}

pub struct ChainLengthIter<'a> {
    txn: &'a SanakirjaTx,
    key: ChainLength,
    cursor: ChainLengthsCursor,
}

impl<'a> Iterator for ChainLengthIter<'a> {
    type Item = &'a BlockId;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next(self.txn).unwrap().and_then(
            |(k, v)| {
                if k == &self.key {
                    Some(v)
                } else {
                    None
                }
            },
        )
    }
}

impl<A: Storable, B: Storable> Storable for Pair<A, B> {
    type PageReferences = core::iter::Chain<A::PageReferences, B::PageReferences>;
    fn page_references(&self) -> Self::PageReferences {
        self.a.page_references().chain(self.b.page_references())
    }
    fn compare<T: sanakirja::LoadPage>(&self, t: &T, b: &Self) -> core::cmp::Ordering {
        match self.a.compare(t, &b.a) {
            core::cmp::Ordering::Equal => self.b.compare(t, &b.b),
            ord => ord,
        }
    }
}

impl<A: Ord + UnsizedStorable, B: Ord + UnsizedStorable> UnsizedStorable for Pair<A, B> {
    const ALIGN: usize = std::mem::align_of::<(A, B)>();

    fn size(&self) -> usize {
        let a = self.a.size();
        let b_off = (a + (B::ALIGN - 1)) & !(B::ALIGN - 1);
        (b_off + self.b.size() + (Self::ALIGN - 1)) & !(Self::ALIGN - 1)
    }
    unsafe fn onpage_size(p: *const u8) -> usize {
        let a = A::onpage_size(p);
        let b_off = (a + (B::ALIGN - 1)) & !(B::ALIGN - 1);
        let b_size = B::onpage_size(p.add(b_off));
        (b_off + b_size + (Self::ALIGN - 1)) & !(Self::ALIGN - 1)
    }
    unsafe fn from_raw_ptr<'a, T>(_: &T, p: *const u8) -> &'a Self {
        &*(p as *const Self)
    }
    unsafe fn write_to_page(&self, p: *mut u8) {
        self.a.write_to_page(p);
        let off = (self.a.size() + (B::ALIGN - 1)) & !(B::ALIGN - 1);
        self.b.write_to_page(p.add(off));
    }
}

fn find_last_record_by<T, K, V>(
    txn: &T,
    tree: &Db<K, V>,
    key: &K,
    max_possible_value: &V,
) -> Option<V>
where
    K: Storable + PartialEq,
    V: Storable + Clone + PartialEq,
    T: ::sanakirja::LoadPage<Error = ::sanakirja::Error>,
{
    let mut cursor = btree::Cursor::new(txn, tree).unwrap();

    cursor.set(txn, key, Some(&max_possible_value)).unwrap();

    if let Some((k, _)) = cursor.prev(txn).unwrap() {
        if k == key {
            cursor.next(txn).unwrap();
        }
    }

    assert!(
        cursor
            .current(txn)
            .unwrap()
            .map(|(_, v)| v != max_possible_value)
            .unwrap_or(true),
        "ran out of sequence numbers"
    );

    cursor
        .current(txn)
        .unwrap()
        .and_then(|(k, v)| if k == key { Some(v.clone()) } else { None })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use chain_addr::{self, Discrimination, Kind};
    use chain_crypto::{AsymmetricKey, Ed25519, KeyPair};
    use chain_impl_mockchain::{
        chaintypes::ConsensusType,
        fragment::ConfigParams,
        transaction::{Input, Output, TransactionSignDataHash, TxBuilder},
    };
    use rand::SeedableRng;
    use std::iter::FromIterator;

    use super::*;

    mod state {
        use std::collections::HashMap;

        use chain_crypto::{PublicKey, SecretKey};
        use chain_impl_mockchain::transaction::{AccountIdentifier, UtxoPointer};
        use rand::{
            distributions::Standard,
            prelude::{Distribution, IteratorRandom},
            RngCore,
        };

        use super::*;
        #[derive(Debug, Clone)]
        pub enum TaggedKeyPair {
            Utxo(KeyPair<Ed25519>),
            Account(KeyPair<Ed25519>),
        }

        impl TaggedKeyPair {
            fn to_address(&self) -> chain_addr::Address {
                let kind = match self {
                    TaggedKeyPair::Utxo(key_pair) => Kind::Single(key_pair.public_key().clone()),
                    TaggedKeyPair::Account(key_pair) => {
                        Kind::Account(key_pair.public_key().clone())
                    }
                };
                chain_addr::Address(Discrimination::Test, kind)
            }
        }

        #[derive(Debug, Clone)]
        pub struct State {
            pub block_id: BlockId,
            pub block0_id: BlockId,
            pub keys: Vec<TaggedKeyPair>,
            pub utxo: BTreeMap<PublicKey<Ed25519>, BTreeSet<UtxoPointer>>,
            pub accounts: BTreeMap<PublicKey<Ed25519>, Value>,
            pub fragments: Vec<Fragment>,
            pub transactions_by_address: BTreeMap<Address, Vec<FragmentId>>,
            pub parent: Option<BlockId>,
        }

        pub fn initial_state(nkeys: u8) -> State {
            let block0_id = StorableHash([1u8; 32]);

            let keys: Vec<TaggedKeyPair> = std::iter::repeat(())
                .enumerate()
                .map(|(i, _)| {
                    let key = SecretKey::<Ed25519>::from_binary(&[i as u8; 32])
                        .unwrap()
                        .into();

                    if i % 2 == 0 {
                        TaggedKeyPair::Utxo(key)
                    } else {
                        TaggedKeyPair::Account(key)
                    }
                })
                .take(nkeys as usize)
                .collect();

            let addresses = keys.iter().map(|key| key.to_address()).collect::<Vec<_>>();

            let mut config_params = ConfigParams::new();

            config_params.push(ConfigParam::ConsensusVersion(ConsensusType::GenesisPraos));
            config_params.push(ConfigParam::Discrimination(
                chain_addr::Discrimination::Test,
            ));
            config_params.push(ConfigParam::EpochStabilityDepth(2));

            let mut initial_fragments = vec![Fragment::Initial(config_params)];

            let mut utxo: BTreeMap<PublicKey<Ed25519>, BTreeSet<UtxoPointer>> = Default::default();
            let mut accounts: BTreeMap<PublicKey<Ed25519>, Value> = Default::default();
            let mut transactions_by_address: BTreeMap<Address, Vec<FragmentId>> = BTreeMap::new();

            for (i, address) in addresses.iter().enumerate() {
                let output = Output::from_address(address.clone(), Value(10000));

                let tx = TxBuilder::new()
                    .set_nopayload()
                    .set_ios(&[], &[output])
                    .set_witnesses_unchecked(&[])
                    .set_payload_auth(&());

                let fragment = Fragment::Transaction(tx);

                transactions_by_address
                    .entry(address.into())
                    .or_default()
                    .insert(0, fragment.id().into());

                match &keys[i] {
                    TaggedKeyPair::Utxo(key_pair) => {
                        let utxo_pointer = UtxoPointer {
                            transaction_id: fragment.id(),
                            output_index: 0,
                            value: Value(100000),
                        };

                        utxo.entry(key_pair.public_key().clone())
                            .or_default()
                            .insert(utxo_pointer);
                    }
                    TaggedKeyPair::Account(key_pair) => {
                        accounts.insert(key_pair.public_key().clone(), Value(100000));
                    }
                }

                initial_fragments.push(fragment);
            }

            State {
                keys,
                accounts,
                utxo,
                transactions_by_address,
                fragments: initial_fragments,
                block_id: block0_id.clone(),
                block0_id,
                parent: None,
            }
        }

        #[derive(Debug, Clone)]
        pub struct TransactionSpec {
            pub from: Vec<(usize, u64)>,
            pub to: Vec<usize>,
        }

        pub fn new_state(prev: &State, block_id: BlockId, spec: TransactionSpec) -> State {
            let mut accounts = prev.accounts.clone();
            let mut utxo = prev.utxo.clone();
            let mut transactions_by_address = prev.transactions_by_address.clone();
            let mut fragments: Vec<Fragment> = vec![];

            let mut inputs = Vec::new();
            let mut outputs = Vec::new();

            let mut add_fragment_to = Vec::new();

            for (id, val) in spec.from.iter().cloned() {
                match &prev.keys[id] {
                    TaggedKeyPair::Utxo(from) => {
                        let utxo_set = utxo.get(from.public_key()).unwrap();

                        if let Some(utxo_pointer) = utxo
                            .get(from.public_key())
                            .unwrap()
                            .iter()
                            .nth(val as usize % utxo_set.len())
                            .cloned()
                        {
                            utxo.entry(from.public_key().clone()).and_modify(|v| {
                                v.remove(&utxo_pointer);
                            });

                            let transfer = (val % utxo_pointer.value.0) + 1;

                            if let Some(change) = utxo_pointer.value.0.checked_sub(transfer) {
                                let change = Output::from_address(
                                    TaggedKeyPair::Utxo(from.clone()).to_address(),
                                    Value(change),
                                );

                                outputs.push(change);
                            }

                            let input = Input::from_utxo(utxo_pointer);

                            inputs.push(input);
                            add_fragment_to.push(id);
                        }
                    }
                    TaggedKeyPair::Account(from) => {
                        let funds = accounts.get_mut(&from.public_key()).unwrap();
                        if funds.0 > 0 {
                            let amount = (val as u64 % funds.0) + 1;

                            *funds = funds.checked_sub(Value(amount)).unwrap();

                            let input = Input::from_account_public_key(
                                from.public_key().clone(),
                                Value(amount),
                            );

                            inputs.push(input);
                            add_fragment_to.push(id);
                        }
                    }
                }
            }

            let total_input = inputs.iter().fold(Value(0), |accum, input| {
                accum.checked_add(input.value()).unwrap()
            });

            let change_output = outputs.iter().fold(Value(0), |accum, output| {
                accum.checked_add(output.value).unwrap()
            });

            let input_to_distribute = total_input.checked_sub(change_output).unwrap();
            let input_per_part = input_to_distribute.0 / (spec.to.len() as u64);

            for id in spec.to {
                let to = &prev.keys[id];

                let output = Output::from_address(to.to_address(), Value(input_per_part));

                outputs.push(output);
                add_fragment_to.push(id);
            }

            let tx_builder = TxBuilder::new()
                .set_nopayload()
                .set_ios(inputs.as_ref(), outputs.as_ref());

            let sign_data_hash: TransactionSignDataHash =
                tx_builder.get_auth_data_for_witness().hash();

            let mut witnesses = Vec::new();

            for (id, _) in spec.from.iter().cloned() {
                match &prev.keys[id] {
                    TaggedKeyPair::Utxo(key_pair) => witnesses.push(Witness::new_utxo(
                        &HeaderId::from_bytes(prev.block0_id.0),
                        &sign_data_hash,
                        |data| key_pair.private_key().sign(data),
                    )),
                    TaggedKeyPair::Account(key_pair) => witnesses.push(Witness::new_account(
                        &HeaderId::from_bytes(prev.block0_id.0),
                        &sign_data_hash,
                        // TODO: the explorer doesn't care about the spending counter, so it's find
                        // to just set it always to 0, but we may need to change this, if we end up
                        // using the actual ledger for this.
                        0u32.into(),
                        |data| key_pair.private_key().sign(data),
                    )),
                }
            }

            let tx = tx_builder
                .set_witnesses_unchecked(witnesses.as_ref())
                .set_payload_auth(&());

            assert_eq!(tx.total_input(), tx.total_output());

            let fragment = Fragment::Transaction(tx);
            fragments.push(fragment.clone());

            for output in outputs {
                match output.address.kind() {
                    Kind::Single(to) => {
                        utxo.entry(to.clone()).and_modify(|v| {
                            v.insert(UtxoPointer {
                                transaction_id: fragment.id(),
                                output_index: 0,
                                value: output.value,
                            });
                        });
                    }
                    Kind::Group(_, _) => {}
                    Kind::Account(to) => {
                        accounts.entry(to.clone()).and_modify(|v| {
                            *v = v.checked_add(output.value).unwrap();
                        });
                    }
                    Kind::Multisig(_) => {}
                    Kind::Script(_) => {}
                }
            }

            for id in add_fragment_to {
                transactions_by_address
                    .entry(prev.keys[id].to_address().into())
                    .or_default()
                    .insert(0, fragment.id().into())
            }

            // let a = &prev.keys[a];
            // let b = &prev.keys[b];

            // match (&a, &b) {
            //     (TaggedKeyPair::Utxo(ref from), to) => {
            //         let utxo_set = utxo.get(from.public_key()).unwrap();

            //         let utxo_pointer = *utxo
            //             .get(from.public_key())
            //             .unwrap()
            //             .iter()
            //             .nth(val % utxo_set.len())
            //             .unwrap();

            //         utxo.entry(from.public_key().clone()).and_modify(|v| {
            //             v.remove(&utxo_pointer);
            //         });

            //         let amount = val as u64 % utxo_pointer.value.0;
            //         let (fragment, outputs) =
            //             make_utxo_tx(from, utxo_pointer, to, amount, &prev.block0_id);

            //         for (i, output) in outputs.iter().enumerate() {
            //             match output.address.kind() {
            //                 Kind::Single(_) => {
            //                     utxo.entry(from.public_key().clone()).and_modify(|v| {
            //                         v.insert(UtxoPointer {
            //                             transaction_id: fragment.id(),
            //                             output_index: i as u8,
            //                             value: outputs[i].value,
            //                         });
            //                     });
            //                 }
            //                 Kind::Group(_, _) => {}
            //                 Kind::Account(_) => match to {
            //                     TaggedKeyPair::Utxo(_) => {
            //                         unreachable!("account key but utxo output")
            //                     }
            //                     TaggedKeyPair::Account(to) => {
            //                         accounts.entry(to.public_key().clone()).and_modify(|v| {
            //                             *v = v.checked_add(output.value).unwrap();
            //                         });
            //                     }
            //                 },
            //                 Kind::Multisig(_) => {}
            //                 Kind::Script(_) => {}
            //             }
            //         }

            //         fragments.push(fragment);
            //     }

            //     (TaggedKeyPair::Account(ref from), to) => {
            //         let funds = accounts.get(&from.public_key()).unwrap();
            //         let amount = val as u64 % funds.0;

            //         let (fragment, output) = make_account_tx(from, &to, amount, &prev.block0_id);

            //         accounts.entry(from.public_key().clone()).and_modify(|v| {
            //             *v = v.checked_sub(Value(amount)).unwrap();
            //         });

            //         match output.address.kind() {
            //             Kind::Single(_) => match to {
            //                 TaggedKeyPair::Utxo(key_pair) => {
            //                     utxo.entry(key_pair.public_key().clone()).and_modify(|v| {
            //                         v.insert(UtxoPointer {
            //                             transaction_id: fragment.id(),
            //                             output_index: 0,
            //                             value: output.value,
            //                         });
            //                     });
            //                 }
            //                 TaggedKeyPair::Account(_) => {
            //                     unreachable!("utxo key but account output");
            //                 }
            //             },
            //             Kind::Group(_, _) => {}
            //             Kind::Account(_) => match to {
            //                 TaggedKeyPair::Utxo(_) => {
            //                     unreachable!("account key but utxo output")
            //                 }
            //                 TaggedKeyPair::Account(to) => {
            //                     accounts.entry(to.public_key().clone()).and_modify(|v| {
            //                         *v = v.checked_add(output.value).unwrap();
            //                     });
            //                 }
            //             },
            //             Kind::Multisig(_) => {}
            //             Kind::Script(_) => {}
            //         }

            //         fragments.push(fragment);
            //     }
            // };

            // for fragment in fragments.iter() {
            //     transactions_by_address
            //         .entry(dbg!(a.to_address().into()))
            //         .or_default()
            //         .insert(0, dbg!(fragment.id().into()));

            //     transactions_by_address
            //         .entry(dbg!(b.to_address().into()))
            //         .or_default()
            //         .insert(0, dbg!(fragment.id().into()));
            // }

            // dbg!("generated new test state---------------------------------------");

            dbg!(State {
                keys: prev.keys.clone(),
                block0_id: prev.block0_id.clone(),
                accounts,
                utxo,
                transactions_by_address,
                fragments,
                parent: Some(prev.block_id.clone()),
                block_id,
            })
        }

        #[test]
        fn test_state() {
            let initial_state = initial_state(10);
            let block1_id = StorableHash([2u8; 32]);
            let block2_id = StorableHash([3u8; 32]);
            let block3_id = StorableHash([4u8; 32]);
            let state1 = new_state(
                &initial_state,
                block1_id,
                TransactionSpec {
                    from: vec![(0, 100)],
                    to: vec![1],
                },
            );

            let state2 = new_state(
                &initial_state,
                block2_id,
                TransactionSpec {
                    from: vec![(3, 100)],
                    to: vec![4],
                },
            );

            let state3 = new_state(
                &state2,
                block3_id,
                TransactionSpec {
                    from: vec![(4, 1000)],
                    to: vec![5],
                },
            );

            dbg!(state1, state2, state3);
            panic!("i want to see");
        }
    }

    mod model {
        use super::*;
        use std::collections::BTreeMap;

        #[derive(Default, Debug)]
        pub struct Model {
            pub states: BTreeMap<BlockId, state::State>,
            pub tips: BTreeSet<(u32, BlockId)>,
            pub fragments: BTreeMap<FragmentId, Fragment>,
            pub blocks_by_chain_length: BTreeMap<u32, BTreeSet<BlockId>>,
            pub block_meta: BTreeMap<BlockId, BlockMeta>,
        }

        impl Model {
            pub const BLOCK0_PARENT_ID: StorableHash = StorableHash([0u8; 32]);
            pub const BLOCK0_ID: StorableHash = StorableHash([1u8; 32]);

            pub fn new() -> Model {
                let initial_state = state::initial_state(10);

                let fragments: BTreeMap<FragmentId, Fragment> = initial_state
                    .fragments
                    .iter()
                    .map(|f| (f.id().into(), f.clone()))
                    .collect();

                let mut blocks_by_chain_length: BTreeMap<u32, BTreeSet<BlockId>> =
                    Default::default();

                blocks_by_chain_length
                    .entry(0)
                    .or_default()
                    .insert(Self::BLOCK0_ID);

                let mut block_meta = BTreeMap::new();

                block_meta.insert(
                    Self::BLOCK0_ID,
                    BlockMeta {
                        chain_length: ChainLength::new(0),
                        date: BlockDate {
                            epoch: EpochNumber::new(0),
                            slot_id: SlotId::new(0),
                        },
                        parent_hash: Self::BLOCK0_PARENT_ID,
                    },
                );

                Model {
                    states: BTreeMap::from_iter(vec![(Self::BLOCK0_ID.clone(), initial_state)]),
                    tips: BTreeSet::from_iter(vec![(0, Self::BLOCK0_ID)]),
                    fragments,
                    blocks_by_chain_length,
                    block_meta,
                }
            }

            pub fn add_block(
                &mut self,
                parent_id: &BlockId,
                block_id: &BlockId,
                block_date: &BlockDate,
                chain_length: &ChainLength,
                spec: state::TransactionSpec,
            ) {
                let parent_chain_length = chain_length.get().checked_sub(1).unwrap();
                self.tips.remove(&(parent_chain_length, parent_id.clone()));

                let previous_state = self
                    .states
                    .get(&parent_id)
                    .cloned()
                    .expect("parent not found");

                let new_state = state::new_state(&previous_state, block_id.clone(), spec);

                for fragment in new_state.fragments.iter() {
                    self.fragments
                        .insert(fragment.id().into(), fragment.clone());
                }

                self.states.insert(block_id.clone(), new_state);

                self.blocks_by_chain_length
                    .entry(chain_length.get())
                    .or_default()
                    .insert(block_id.clone());

                self.block_meta.insert(
                    block_id.clone(),
                    BlockMeta {
                        chain_length: chain_length.clone(),
                        date: block_date.clone(),
                        parent_hash: parent_id.clone(),
                    },
                );

                self.tips.insert((chain_length.get(), block_id.clone()));
            }

            pub fn get_branches(&self) -> Vec<BlockId> {
                self.tips.iter().map(|(_, v)| v.clone()).collect()
            }

            pub fn get_state_refs(&self) -> impl Iterator<Item = (&BlockId, &state::State)> {
                self.states.iter()
            }

            pub fn get_blocks_by_chain_length(
                &self,
                chain_length: &ChainLength,
            ) -> Option<&BTreeSet<BlockId>> {
                self.blocks_by_chain_length.get(&chain_length.get())
            }

            pub fn get_block_meta(&self, block_id: &BlockId) -> Option<&BlockMeta> {
                self.block_meta.get(block_id)
            }

            pub fn get_fragment(&self, fragment_id: &FragmentId) -> Option<&Fragment> {
                self.fragments.get(fragment_id)
            }
        }
    }

    #[test]
    fn sanakirja_test() {
        let pristine = Pristine::new_anon().unwrap();
        let mut model = model::Model::new();

        let mut mut_tx = pristine.mut_txn_begin().unwrap();

        let block0_id = StorableHash([1u8; 32]);
        let block1_id = StorableHash([2u8; 32]);
        let block1_date = BlockDate {
            epoch: EpochNumber::new(0),
            slot_id: SlotId::new(1),
        };

        let state = model.states.get(&model::Model::BLOCK0_ID).unwrap();

        mut_tx
            .add_block0(
                &model::Model::BLOCK0_PARENT_ID,
                &model::Model::BLOCK0_ID,
                state.fragments.clone().into_iter(),
            )
            .unwrap();

        model.add_block(
            &block0_id,
            &block1_id,
            &block1_date,
            &ChainLength::new(1),
            state::TransactionSpec {
                from: vec![(0, 5000)],
                to: vec![3],
            },
        );

        let branch1_id = StorableHash([2u8; 32]);
        let branch2_id = StorableHash([3u8; 32]);
        let branch3_id = StorableHash([4u8; 32]);

        let branch_config: BTreeMap<BlockId, (BlockId, Vec<state::TransactionSpec>)> =
            BTreeMap::from_iter([
                (
                    branch1_id.clone(),
                    (
                        block0_id.clone(),
                        vec![state::TransactionSpec {
                            from: vec![(0, 140), (5, 2500)],
                            to: vec![3, 6],
                        }],
                    ),
                ),
                (
                    branch2_id.clone(),
                    (
                        branch1_id.clone(),
                        vec![state::TransactionSpec {
                            from: vec![(1, 3000), (4, 5600)],
                            to: vec![3, 2],
                        }],
                    ),
                ),
                (
                    branch3_id.clone(),
                    (
                        block0_id.clone(),
                        vec![state::TransactionSpec {
                            from: vec![(0, 50000)],
                            to: vec![7],
                        }],
                    ),
                ),
            ]);

        for (branch_id, (parent, spec)) in branch_config {
            let block_date = BlockDate {
                epoch: B32::new(0),
                slot_id: B32::new(1),
            };

            model.add_block(
                &parent,
                &branch_id,
                &block_date,
                &ChainLength::new(1),
                spec[0].clone(),
            );

            let fragments = model.states.get(&branch_id).unwrap().fragments.clone();

            mut_tx
                .add_block(
                    &parent,
                    &branch_id,
                    ChainLength::new(1),
                    block_date,
                    fragments,
                )
                .unwrap();
        }

        mut_tx.commit();

        let txn = pristine.txn_begin().unwrap();

        for (branch_id, branch) in model.get_state_refs() {
            for (address, fragments) in branch.transactions_by_address.iter() {
                assert_eq!(
                    txn.get_transactions_by_address(&branch_id, dbg!(address), None)
                        .unwrap()
                        .map(|v| v.collect::<Vec<FragmentId>>()),
                    Some(fragments.clone()),
                );
            }
        }

        assert_eq!(
            txn.get_branches().collect::<Vec<BlockId>>(),
            model.get_branches()
        );

        for block in [&block0_id, &branch1_id, &branch2_id, &branch3_id] {
            assert_eq!(
                txn.get_block_fragments(block).collect::<Vec<FragmentId>>(),
                model
                    .states
                    .get(block)
                    .unwrap()
                    .fragments
                    .iter()
                    .map(|f| f.id().into())
                    .collect::<Vec<FragmentId>>()
            );

            assert_eq!(txn.get_block_meta(block), model.get_block_meta(block));

            for fragment_id in txn.get_block_fragments(block) {
                let fragment = model.get_fragment(&fragment_id).unwrap();

                match fragment {
                    Fragment::Initial(_) => {}
                    Fragment::UpdateProposal(_) => {}
                    Fragment::UpdateVote(_) => {}
                    Fragment::OldUtxoDeclaration(_) => {}
                    Fragment::Transaction(tx) => assert_transaction(&txn, &fragment_id, tx),
                    Fragment::OwnerStakeDelegation(tx) => {
                        assert_transaction(&txn, &fragment_id, tx)
                    }
                    Fragment::StakeDelegation(tx) => assert_transaction(&txn, &fragment_id, tx),
                    Fragment::PoolRegistration(tx) => assert_transaction(&txn, &fragment_id, tx),
                    Fragment::PoolRetirement(tx) => assert_transaction(&txn, &fragment_id, tx),
                    Fragment::PoolUpdate(tx) => assert_transaction(&txn, &fragment_id, tx),
                    Fragment::VotePlan(tx) => assert_transaction(&txn, &fragment_id, tx),
                    Fragment::VoteCast(tx) => assert_transaction(&txn, &fragment_id, tx),
                    Fragment::VoteTally(tx) => assert_transaction(&txn, &fragment_id, tx),
                    Fragment::EncryptedVoteTally(tx) => assert_transaction(&txn, &fragment_id, tx),
                }

                fn assert_transaction<P>(
                    txn: &Txn,
                    fragment_id: &FragmentId,
                    tx: &transaction::Transaction<P>,
                ) {
                    let tx = tx.as_slice();
                    for (real_input, explorer_input) in tx
                        .inputs()
                        .iter()
                        .zip(txn.get_fragment_inputs(&fragment_id, None))
                    {
                        assert_eq!(real_input, explorer_input.into());
                    }

                    for (real_output, explorer_output) in tx
                        .outputs()
                        .iter()
                        .zip(txn.get_fragment_outputs(&fragment_id, None))
                    {
                        assert_eq!(real_output, explorer_output.into());
                    }
                }
            }
        }

        for chain_length in 0..=3 {
            assert_eq!(
                txn.get_blocks_by_chain_length(&ChainLength::new(chain_length))
                    .cloned()
                    .collect::<BTreeSet<_>>(),
                model
                    .get_blocks_by_chain_length(&ChainLength::new(chain_length))
                    .map(Clone::clone)
                    .unwrap_or_default(),
            );
        }

        // pub type VotePlans = UDb<VotePlanId, VotePlanMeta>;
        // pub type VotePlanProposals = UDb<VotePlanId, Pair<u8, ExplorerVoteProposal>>;
        // pub type StakePools = UDb<PoolId, StakePoolMeta>;
    }
}
