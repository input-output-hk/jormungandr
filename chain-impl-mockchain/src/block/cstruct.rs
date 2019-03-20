use std::mem::size_of;

type Version = u16;
type ContentSize = u32;
type DateEpoch = u32;
type DateSlotid = u32;
type Height = u32;
type ContentHash = [u8; 32];
type ParentHash = [u8; 32];

type BftLeaderId = [u8; 32];
type BftSignature = [u8; 64];

type GpNodeId = [u8; 32];
type GpVrfProof = [u8; 96];

// common parts
const BLOCK_OFFSET_VERSION: usize = 0;
const BLOCK_OFFSET_CONTENT_SIZE: usize = BLOCK_OFFSET_VERSION + size_of::<Version>();
const BLOCK_OFFSET_DATE_EPOCH: usize = BLOCK_OFFSET_CONTENT_SIZE + size_of::<ContentSize>();
const BLOCK_OFFSET_DATE_SLOTID: usize = BLOCK_OFFSET_DATE_EPOCH + size_of::<DateEpoch>();
const BLOCK_OFFSET_HEIGHT: usize = BLOCK_OFFSET_DATE_SLOTID + size_of::<DateSlotid>();
const BLOCK_OFFSET_CONTENT_HASH: usize = BLOCK_OFFSET_HEIGHT + size_of::<Height>();
const BLOCK_OFFSET_PARENT_HASH: usize = BLOCK_OFFSET_HEIGHT + size_of::<ContentHash>();

const BLOCK_COMMON_SIZE: usize = BLOCK_OFFSET_PARENT_HASH + size_of::<ParentHash>();

// BFT
const BLOCK_OFFSET_BFT_LEADER_ID: usize = BLOCK_COMMON_SIZE;
const BLOCK_OFFSET_BFT_SIGNATURE: usize = BLOCK_OFFSET_BFT_LEADER_ID + size_of::<BftLeaderId>();

const BLOCK_BFT_SIZE: usize = BLOCK_OFFSET_BFT_SIGNATURE + size_of::<BftSignature>();

// GenesisPraos
const BLOCK_OFFSET_GP_ID: usize = BLOCK_COMMON_SIZE;
const BLOCK_OFFSET_GP_VRF_PROOF: usize = BLOCK_OFFSET_GP_ID + size_of::<GpNodeId>();
const BLOCK_OFFSET_GP_KES_SIG: usize = BLOCK_OFFSET_GP_VRF_PROOF + size_of::<GpVrfProof>();
