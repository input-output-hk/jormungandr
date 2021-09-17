use super::endian::{B32, L64};
use chain_impl_mockchain::header::HeaderId;
use sanakirja::{direct_repr, Storable, UnsizedStorable};
use zerocopy::{AsBytes, FromBytes};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct AccountId(pub [u8; chain_impl_mockchain::transaction::INPUT_PTR_SIZE]);
direct_repr!(AccountId);

impl std::fmt::Debug for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

pub type ProposalIndex = u8;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProposalId {
    pub vote_plan: VotePlanId,
    pub index: ProposalIndex,
}
direct_repr!(ProposalId);

pub type BlockId = StorableHash;
pub type FragmentId = StorableHash;
pub type VotePlanId = StorableHash;

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, AsBytes, FromBytes)]
#[cfg_attr(test, derive(Hash))]
#[repr(C)]
pub struct StorableHash(pub [u8; 32]);

impl StorableHash {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl std::fmt::Display for StorableHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

direct_repr!(StorableHash);

impl StorableHash {
    pub const MIN: Self = StorableHash([0x00; 32]);
    pub const MAX: Self = StorableHash([0xff; 32]);
}

impl From<chain_impl_mockchain::key::Hash> for StorableHash {
    fn from(id: chain_impl_mockchain::key::Hash) -> Self {
        let bytes: [u8; 32] = id.into();

        Self(bytes)
    }
}

impl From<StorableHash> for chain_impl_mockchain::key::Hash {
    fn from(val: StorableHash) -> Self {
        HeaderId::from(val.0)
    }
}

impl From<chain_impl_mockchain::certificate::VotePlanId> for StorableHash {
    fn from(id: chain_impl_mockchain::certificate::VotePlanId) -> Self {
        let bytes: [u8; 32] = id.into();

        Self(bytes)
    }
}

impl From<[u8; 32]> for StorableHash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<StorableHash> for [u8; 32] {
    fn from(wrapper: StorableHash) -> Self {
        wrapper.0
    }
}

impl std::fmt::Debug for StorableHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

pub type SlotId = B32;
pub type EpochNumber = B32;

#[derive(Debug, Clone, Copy, AsBytes, FromBytes, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct ChainLength(pub(super) B32);

impl ChainLength {
    pub const MAX: ChainLength = ChainLength(B32(zerocopy::U32::<byteorder::BigEndian>::MAX_VALUE));
    pub const MIN: ChainLength = ChainLength(B32(zerocopy::U32::<byteorder::BigEndian>::ZERO));

    pub fn new(n: u32) -> Self {
        Self(B32::new(n))
    }

    pub fn get(&self) -> u32 {
        self.0.get()
    }
}

direct_repr!(ChainLength);

impl From<chain_impl_mockchain::block::ChainLength> for ChainLength {
    fn from(c: chain_impl_mockchain::block::ChainLength) -> Self {
        Self(B32::new(u32::from(c)))
    }
}

impl From<ChainLength> for chain_impl_mockchain::block::ChainLength {
    fn from(c: ChainLength) -> Self {
        c.get().into()
    }
}

impl From<&ChainLength> for u32 {
    fn from(n: &ChainLength) -> Self {
        n.0.get()
    }
}

impl From<ChainLength> for u32 {
    fn from(n: ChainLength) -> Self {
        n.0.get()
    }
}

impl From<u32> for ChainLength {
    fn from(n: u32) -> Self {
        ChainLength::new(n)
    }
}

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

pub type PoolId = StorableHash;

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

impl From<PayloadType> for chain_impl_mockchain::vote::PayloadType {
    fn from(p: PayloadType) -> Self {
        match p {
            PayloadType::Public => chain_impl_mockchain::vote::PayloadType::Public,
            PayloadType::Private => chain_impl_mockchain::vote::PayloadType::Private,
        }
    }
}

pub type ExternalProposalId = StorableHash;
pub type Options = u8;

#[derive(Clone, Debug, FromBytes, AsBytes, PartialEq, Eq, Ord, PartialOrd)]
#[repr(C)]
pub struct ExplorerVoteProposal {
    pub proposal_id: ExternalProposalId,
    pub options: Options,
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

pub type Choice = u8;

pub type Stake = L64;

pub const MAX_ADDRESS_SIZE: usize = chain_addr::ADDR_SIZE_GROUP;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, AsBytes, FromBytes)]
#[repr(C)]
pub struct Address(pub [u8; MAX_ADDRESS_SIZE]);

impl Address {
    pub const MIN: Address = Address([0u8; MAX_ADDRESS_SIZE]);
    pub const MAX: Address = Address([255u8; MAX_ADDRESS_SIZE]);
}

direct_repr!(Address);

impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.0))
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
pub enum InputType {
    Utxo = 0x00,
    // Notes:
    // the original (on chain) type has only two discriminant values.
    // the witness type is used to decide how to interpret the bytes in `input_ptr`, because the
    // explorer doesn't store the witnesses, we need to save that metadata somewhere, that's the
    // reason for the extra variant. It could be stored externally, but it would take more space
    // for all inputs (unless is stored in a separate btree, but that uses a lot of space too).
    AccountSingle = 0xfe,
    AccountMulti = 0xff,
}

direct_repr!(TransactionInput);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, AsBytes, FromBytes)]
#[repr(C)]
pub struct TransactionOutput {
    pub address: Address,
    pub value: L64,
}

direct_repr!(TransactionOutput);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(C)]
pub struct TransactionCertificate {
    pub(crate) tag: CertificateTag,
    pub(crate) cert: SerializedCertificate,
}

direct_repr!(TransactionCertificate);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(u8)]
pub enum CertificateTag {
    VotePlan = 0,
    PublicVoteCast = 1,
    PrivateVoteCast = 2,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(C)]
pub struct SerializedCertificate(
    [u8; max(
        std::mem::size_of::<VotePlanId>(),
        max(
            std::mem::size_of::<PublicVoteCast>(),
            std::mem::size_of::<PrivateVoteCast>(),
        ),
    )],
);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(C)]
pub struct PublicVoteCast {
    pub vote_plan_id: VotePlanId,
    pub proposal_index: u8,
    pub choice: Choice,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, AsBytes)]
#[repr(C)]
pub struct PrivateVoteCast {
    pub vote_plan_id: VotePlanId,
    pub proposal_index: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, AsBytes)]
#[repr(C)]
pub struct VotePlanMeta {
    pub vote_start: BlockDate,
    pub vote_end: BlockDate,
    pub committee_end: BlockDate,
    pub payload_type: PayloadType,
}

direct_repr!(VotePlanMeta);

const fn max(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}
