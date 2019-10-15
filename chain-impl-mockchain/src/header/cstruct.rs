// lowlevel header binary accessors, use module qualified and fundamentally allow to do invalid construction
#![allow(dead_code)]

use std::mem::size_of;

// ************************************************************************
// Offset and Basic Types
// ************************************************************************

pub(super) type Version = u16;
pub(super) type ContentSize = u32;
pub(super) type DateEpoch = u32;
pub(super) type DateSlotid = u32;
pub(super) type Height = u32;
pub(super) type ContentHash = [u8; 32];
pub(super) type ParentHash = [u8; 32];

pub(super) type BftLeaderId = [u8; 32];
pub(super) type BftSignature = [u8; 64];

pub(super) type GpNodeId = [u8; 32];
pub(super) type GpVrfProof = [u8; 96];
pub(super) type GpKesSignature = [u8; 484];

// common parts
const HEADER_OFFSET_VERSION: usize = 0;
const HEADER_OFFSET_CONTENT_SIZE: usize = HEADER_OFFSET_VERSION + size_of::<Version>();
const HEADER_OFFSET_DATE_EPOCH: usize = HEADER_OFFSET_CONTENT_SIZE + size_of::<ContentSize>();
const HEADER_OFFSET_DATE_SLOTID: usize = HEADER_OFFSET_DATE_EPOCH + size_of::<DateEpoch>();
const HEADER_OFFSET_HEIGHT: usize = HEADER_OFFSET_DATE_SLOTID + size_of::<DateSlotid>();
const HEADER_OFFSET_CONTENT_HASH: usize = HEADER_OFFSET_HEIGHT + size_of::<Height>();
const HEADER_OFFSET_PARENT_HASH: usize = HEADER_OFFSET_CONTENT_HASH + size_of::<ContentHash>();

pub const HEADER_COMMON_SIZE: usize = HEADER_OFFSET_PARENT_HASH + size_of::<ParentHash>();

// BFT
const HEADER_OFFSET_BFT_LEADER_ID: usize = HEADER_COMMON_SIZE;
const HEADER_OFFSET_BFT_SIGNATURE: usize = HEADER_OFFSET_BFT_LEADER_ID + size_of::<BftLeaderId>();

pub const HEADER_BFT_SIZE: usize = HEADER_OFFSET_BFT_SIGNATURE + size_of::<BftSignature>();

pub const HEADER_BFT_AUTHED_SIZE: usize = HEADER_OFFSET_BFT_SIGNATURE;

// GenesisPraos
const HEADER_OFFSET_GP_ID: usize = HEADER_COMMON_SIZE;
const HEADER_OFFSET_GP_VRF_PROOF: usize = HEADER_OFFSET_GP_ID + size_of::<GpNodeId>();
const HEADER_OFFSET_GP_KES_SIG: usize = HEADER_OFFSET_GP_VRF_PROOF + size_of::<GpVrfProof>();

pub const HEADER_GP_SIZE: usize = HEADER_OFFSET_GP_KES_SIG + size_of::<GpKesSignature>();

pub const HEADER_GP_AUTHED_SIZE: usize = HEADER_OFFSET_GP_KES_SIG;

pub const HEADER_MIN_KNOWN_SIZE: usize = HEADER_COMMON_SIZE;
pub const HEADER_MAX_KNOWN_SIZE: usize = HEADER_GP_SIZE;

// ************************************************************************
// Header union construction & accessors
// ************************************************************************

pub(super) type HeaderUnsigned = [u8; HEADER_COMMON_SIZE];
pub(super) type HeaderBFT = [u8; HEADER_BFT_SIZE];
pub(super) type HeaderGP = [u8; HEADER_GP_SIZE];

pub(super) union Header {
    unsigned: HeaderUnsigned,
    bft: HeaderBFT,
    gp: HeaderGP,
}

impl Clone for Header {
    fn clone(&self) -> Self {
        let mut gp = [0u8; HEADER_GP_SIZE];
        gp[..].copy_from_slice(unsafe { &self.gp[..] });
        Header { gp }
    }
}

impl PartialEq for Header {
    fn eq(&self, other: &Self) -> bool {
        unsafe { &self.gp[..] == &other.gp[..] }
    }
}
impl Eq for Header {}

pub(super) const VERSION_UNSIGNED: Version = 0;
pub(super) const VERSION_BFT: Version = 1;
pub(super) const VERSION_GP: Version = 2;

pub struct HeaderSlice<'a>(&'a [u8]);

impl Header {
    pub fn version(&self) -> Version {
        let mut buf = [0u8; size_of::<Version>()];
        let s = unsafe { &self.unsigned[HEADER_OFFSET_VERSION..HEADER_OFFSET_CONTENT_SIZE] };
        buf.copy_from_slice(s);
        Version::from_be_bytes(buf)
    }

    pub fn as_slice<'a>(&'a self) -> HeaderSlice<'a> {
        match self.version() {
            VERSION_UNSIGNED => unsafe { HeaderSlice(&self.unsigned[..]) },
            VERSION_BFT => unsafe { HeaderSlice(&self.bft[..]) },
            VERSION_GP => unsafe { HeaderSlice(&self.gp[..]) },
            _ => panic!("Header: cstruct: as slice with undefined version"),
        }
    }

    pub(self) fn as_slice_mut<'a>(&'a mut self) -> &mut [u8] {
        match self.version() {
            VERSION_UNSIGNED => unsafe { &mut self.unsigned[..] },
            VERSION_BFT => unsafe { &mut self.bft[..] },
            VERSION_GP => unsafe { &mut self.gp[..] },
            _ => panic!("Header: cstruct: as slice mut with undefined version"),
        }
    }

    pub fn new(version: Version) -> Header {
        let gp = [0u8; HEADER_GP_SIZE];
        let mut hdr = Header { gp };
        hdr.set_version(version);
        hdr
    }

    pub fn set_version(&mut self, s: Version) {
        let sbuf = s.to_be_bytes();
        unsafe {
            self.unsigned[HEADER_OFFSET_VERSION..HEADER_OFFSET_CONTENT_SIZE]
                .copy_from_slice(&sbuf[..])
        }
    }

    pub fn set_content_size(&mut self, s: ContentSize) {
        let sbuf = s.to_be_bytes();
        unsafe {
            self.unsigned[HEADER_OFFSET_CONTENT_SIZE..HEADER_OFFSET_DATE_EPOCH]
                .copy_from_slice(&sbuf[..])
        }
    }

    pub fn set_date_epoch(&mut self, s: DateEpoch) {
        let sbuf = s.to_be_bytes();
        unsafe {
            self.unsigned[HEADER_OFFSET_DATE_EPOCH..HEADER_OFFSET_DATE_SLOTID]
                .copy_from_slice(&sbuf[..])
        }
    }

    pub fn set_date_slotid(&mut self, s: DateSlotid) {
        let sbuf = s.to_be_bytes();
        unsafe {
            self.unsigned[HEADER_OFFSET_DATE_SLOTID..HEADER_OFFSET_HEIGHT]
                .copy_from_slice(&sbuf[..])
        }
    }

    pub fn set_height(&mut self, s: Height) {
        let sbuf = s.to_be_bytes();
        unsafe {
            self.unsigned[HEADER_OFFSET_HEIGHT..HEADER_OFFSET_CONTENT_HASH]
                .copy_from_slice(&sbuf[..])
        }
    }

    pub fn set_content_hash(&mut self, s: &ContentHash) {
        unsafe {
            self.unsigned[HEADER_OFFSET_CONTENT_HASH..HEADER_OFFSET_PARENT_HASH]
                .copy_from_slice(&s[..])
        }
    }

    pub fn set_parent_hash(&mut self, s: &ParentHash) {
        unsafe {
            self.unsigned[HEADER_OFFSET_PARENT_HASH..HEADER_COMMON_SIZE].copy_from_slice(&s[..])
        }
    }

    #[allow(dead_code)]
    pub fn set_bft_leader_id(&mut self, s: &BftLeaderId) {
        assert_eq!(self.version(), VERSION_BFT);
        unsafe {
            self.bft[HEADER_OFFSET_BFT_LEADER_ID..HEADER_OFFSET_BFT_SIGNATURE]
                .copy_from_slice(&s[..])
        }
    }

    pub fn set_bft_leader_id_slice(&mut self, s: &[u8]) {
        assert_eq!(self.version(), VERSION_BFT);
        assert_eq!(s.len(), size_of::<BftLeaderId>());
        unsafe {
            self.bft[HEADER_OFFSET_BFT_LEADER_ID..HEADER_OFFSET_BFT_SIGNATURE].copy_from_slice(s)
        }
    }

    #[allow(dead_code)]
    pub fn set_bft_signature(&mut self, s: &BftSignature) {
        assert_eq!(self.version(), VERSION_BFT);
        unsafe { self.bft[HEADER_OFFSET_BFT_SIGNATURE..HEADER_BFT_SIZE].copy_from_slice(&s[..]) }
    }

    pub fn set_bft_signature_slice(&mut self, s: &[u8]) {
        assert_eq!(self.version(), VERSION_BFT);
        assert_eq!(s.len(), size_of::<BftSignature>());
        unsafe { self.bft[HEADER_OFFSET_BFT_SIGNATURE..HEADER_BFT_SIZE].copy_from_slice(s) }
    }

    pub fn set_gp_node_id(&mut self, s: &GpNodeId) {
        assert_eq!(self.version(), VERSION_GP);
        unsafe { self.gp[HEADER_OFFSET_GP_ID..HEADER_OFFSET_GP_VRF_PROOF].copy_from_slice(&s[..]) }
    }

    #[allow(dead_code)]
    pub fn set_gp_node_id_slice(&mut self, s: &[u8]) {
        assert_eq!(self.version(), VERSION_GP);
        assert_eq!(s.len(), size_of::<GpNodeId>());
        unsafe { self.gp[HEADER_OFFSET_GP_ID..HEADER_OFFSET_GP_VRF_PROOF].copy_from_slice(s) }
    }

    pub fn set_gp_vrf_proof(&mut self, s: &GpVrfProof) {
        assert_eq!(self.version(), VERSION_GP);
        unsafe {
            self.gp[HEADER_OFFSET_GP_VRF_PROOF..HEADER_OFFSET_GP_KES_SIG].copy_from_slice(&s[..])
        }
    }

    #[allow(dead_code)]
    pub fn set_gp_vrf_proof_slice(&mut self, s: &[u8]) {
        assert_eq!(self.version(), VERSION_GP);
        assert_eq!(s.len(), size_of::<GpVrfProof>());
        unsafe { self.gp[HEADER_OFFSET_GP_VRF_PROOF..HEADER_OFFSET_GP_KES_SIG].copy_from_slice(s) }
    }

    #[allow(dead_code)]
    pub fn set_gp_kes_signature(&mut self, s: &GpKesSignature) {
        assert_eq!(self.version(), VERSION_GP);
        unsafe { self.gp[HEADER_OFFSET_GP_KES_SIG..HEADER_GP_SIZE].copy_from_slice(&s[..]) }
    }

    pub fn set_gp_kes_signature_slice(&mut self, s: &[u8]) {
        assert_eq!(self.version(), VERSION_GP);
        assert_eq!(s.len(), size_of::<GpKesSignature>());
        unsafe { self.gp[HEADER_OFFSET_GP_KES_SIG..HEADER_GP_SIZE].copy_from_slice(s) }
    }
}

#[derive(Debug, Clone)]
pub enum HeaderError {
    InvalidSize,
    UnknownVersion,
    SizeMismatch { expected: usize, got: usize },
}

impl<'a> HeaderSlice<'a> {
    pub fn from_slice(slice: &'a [u8]) -> Result<Self, HeaderError> {
        let len = slice.len();
        if len < HEADER_MIN_KNOWN_SIZE {
            return Err(HeaderError::InvalidSize);
        }
        if len > HEADER_MAX_KNOWN_SIZE {
            return Err(HeaderError::InvalidSize);
        }

        let hdr = HeaderSlice(slice);
        match hdr.version() {
            VERSION_UNSIGNED => {
                if len != HEADER_COMMON_SIZE {
                    return Err(HeaderError::SizeMismatch {
                        expected: HEADER_COMMON_SIZE,
                        got: len,
                    });
                }
                Ok(hdr)
            }
            VERSION_BFT => {
                if len != HEADER_BFT_SIZE {
                    return Err(HeaderError::SizeMismatch {
                        expected: HEADER_BFT_SIZE,
                        got: len,
                    });
                }
                Ok(hdr)
            }
            VERSION_GP => {
                if len != HEADER_GP_SIZE {
                    return Err(HeaderError::SizeMismatch {
                        expected: HEADER_GP_SIZE,
                        got: len,
                    });
                }
                Ok(hdr)
            }
            _ => Err(HeaderError::UnknownVersion),
        }
    }

    pub fn as_slice(self) -> &'a [u8] {
        &self.0[..]
    }

    pub(super) fn into_owned(&self) -> Header {
        let mut new = Header::new(self.version());
        new.as_slice_mut().copy_from_slice(&self.0);
        new
    }

    pub fn version(&self) -> Version {
        let mut buf = [0u8; size_of::<Version>()];
        buf.copy_from_slice(&self.0[HEADER_OFFSET_VERSION..HEADER_OFFSET_CONTENT_SIZE]);
        Version::from_be_bytes(buf)
    }

    pub fn content_size(&self) -> ContentSize {
        let mut buf = [0u8; size_of::<ContentSize>()];
        buf.copy_from_slice(&self.0[HEADER_OFFSET_CONTENT_SIZE..HEADER_OFFSET_DATE_EPOCH]);
        ContentSize::from_be_bytes(buf)
    }

    pub fn date_epoch(&self) -> DateEpoch {
        let mut buf = [0u8; size_of::<DateEpoch>()];
        buf.copy_from_slice(&self.0[HEADER_OFFSET_DATE_EPOCH..HEADER_OFFSET_DATE_SLOTID]);
        DateEpoch::from_be_bytes(buf)
    }

    pub fn date_slotid(&self) -> DateSlotid {
        let mut buf = [0u8; size_of::<DateSlotid>()];
        buf.copy_from_slice(&self.0[HEADER_OFFSET_DATE_SLOTID..HEADER_OFFSET_HEIGHT]);
        DateSlotid::from_be_bytes(buf)
    }

    pub fn height(&self) -> Height {
        let mut buf = [0u8; size_of::<Height>()];
        buf.copy_from_slice(&self.0[HEADER_OFFSET_HEIGHT..HEADER_OFFSET_CONTENT_HASH]);
        Height::from_be_bytes(buf)
    }

    pub fn content_hash_ref(&self) -> &[u8] {
        &self.0[HEADER_OFFSET_CONTENT_HASH..HEADER_OFFSET_PARENT_HASH]
    }

    pub fn content_hash(&self) -> ContentHash {
        let mut buf = [0u8; size_of::<ContentHash>()];
        buf.copy_from_slice(self.content_hash_ref());
        buf
    }

    pub fn parent_hash_ref(&self) -> &[u8] {
        &self.0[HEADER_OFFSET_PARENT_HASH..HEADER_COMMON_SIZE]
    }

    pub fn parent_hash(&self) -> ParentHash {
        let mut buf = [0u8; size_of::<ParentHash>()];
        buf.copy_from_slice(self.parent_hash_ref());
        buf
    }

    pub fn bft_leader_id_ref(&self) -> &[u8] {
        assert_eq!(self.version(), VERSION_BFT);
        &self.0[HEADER_OFFSET_BFT_LEADER_ID..HEADER_OFFSET_BFT_SIGNATURE]
    }

    pub fn bft_leader_id(&self) -> BftLeaderId {
        let mut buf = [0u8; size_of::<BftLeaderId>()];
        buf.copy_from_slice(self.bft_leader_id_ref());
        buf
    }

    pub fn bft_signature_ref(&self) -> &[u8] {
        assert_eq!(self.version(), VERSION_BFT);
        &self.0[HEADER_OFFSET_BFT_SIGNATURE..HEADER_BFT_SIZE]
    }

    pub fn bft_signature(&self) -> BftSignature {
        let mut buf = [0u8; size_of::<BftSignature>()];
        buf.copy_from_slice(self.bft_signature_ref());
        buf
    }

    pub fn gp_node_id_ref(&self) -> &[u8] {
        assert_eq!(self.version(), VERSION_GP);
        &self.0[HEADER_OFFSET_GP_ID..HEADER_OFFSET_GP_VRF_PROOF]
    }

    pub fn gp_node_id(&self) -> GpNodeId {
        let mut buf = [0u8; size_of::<GpNodeId>()];
        buf.copy_from_slice(self.gp_node_id_ref());
        buf
    }

    pub fn gp_vrf_proof_ref(&self) -> &[u8] {
        assert_eq!(self.version(), VERSION_GP);
        &self.0[HEADER_OFFSET_GP_VRF_PROOF..HEADER_OFFSET_GP_KES_SIG]
    }

    pub fn gp_vrf_proof(&self) -> GpVrfProof {
        let mut buf = [0u8; size_of::<GpVrfProof>()];
        buf.copy_from_slice(self.gp_vrf_proof_ref());
        buf
    }

    pub fn gp_kes_signature_ref(&self) -> &[u8] {
        assert_eq!(self.version(), VERSION_GP);
        &self.0[HEADER_OFFSET_GP_KES_SIG..HEADER_GP_SIZE]
    }

    pub fn gp_kes_signature(&self) -> GpKesSignature {
        let mut buf = [0u8; size_of::<GpKesSignature>()];
        buf.copy_from_slice(self.gp_kes_signature_ref());
        buf
    }

    pub fn slice_bft_auth(self) -> &'a [u8] {
        assert_eq!(self.version(), VERSION_BFT);
        &self.0[0..HEADER_BFT_AUTHED_SIZE]
    }

    pub fn slice_gp_auth(self) -> &'a [u8] {
        assert_eq!(self.version(), VERSION_GP);
        &self.0[0..HEADER_GP_AUTHED_SIZE]
    }
}
