use crate::{
    block::{
        Block, BlockVersion, BLOCK_VERSION_CONSENSUS_BFT, BLOCK_VERSION_CONSENSUS_GENESIS_PRAOS,
        BLOCK_VERSION_CONSENSUS_NONE,
    },
    state::State,
};
use chain_core::property::Block as _;
use chain_crypto::algorithms::vrf::vrf::{self, ProvenOutputSeed};
use chain_crypto::{Curve25519_2HashDH, Ed25519Extended, FakeMMM, PublicKey, SecretKey};

pub mod bft;
pub mod genesis;
pub mod none;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    Failure,
    NoLeaderForThisSlot,
    IncompatibleBlockVersion,
    IncompatibleLeadershipMode,
    InvalidLeader,
    InvalidLeaderSignature,
    InvalidBlockMessage,
    InvalidStateUpdate,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    cause: Option<Box<dyn std::error::Error>>,
}

/// Verification type for when validating a block
#[derive(Debug)]
pub enum Verification {
    Success,
    Failure(Error),
}

macro_rules! try_check {
    ($x:expr) => {
        if $x.failure() {
            return $x;
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LeaderId {
    None,
    Bft(bft::LeaderId),
    GenesisPraos(genesis::GenesisPraosLeader),
}

pub enum Leader {
    None,
    BftLeader(SecretKey<Ed25519Extended>),
    GenesisPraos(
        SecretKey<FakeMMM>,
        SecretKey<Curve25519_2HashDH>,
        ProvenOutputSeed,
    ),
}

enum Inner {
    None,
    Bft(bft::BftLeaderSelection),
    // GenesisPraos,
}

pub struct Leadership {
    inner: Inner,
}

impl Inner {
    #[inline]
    fn verify_version(&self, block_version: BlockVersion) -> Verification {
        match self {
            Inner::None if block_version == BLOCK_VERSION_CONSENSUS_NONE => Verification::Success,
            Inner::Bft(_) if block_version == BLOCK_VERSION_CONSENSUS_BFT => Verification::Success,
            _ => Verification::Failure(Error::new(ErrorKind::IncompatibleBlockVersion)),
        }
    }
}

impl Leadership {
    pub fn new(state: &State) -> Result<Self, Error> {
        unimplemented!()
    }

    pub fn verify(&self, block: &Block) -> Verification {
        try_check!(self.inner.verify_version(block.version()));

        // let check_leader = self.inner.check_leader(&block.header);
        Verification::Success
    }
}

impl chain_core::property::LeaderId for LeaderId {}

impl Verification {
    #[inline]
    pub fn into_error(self) -> Result<(), Error> {
        match self {
            Verification::Success => Ok(()),
            Verification::Failure(err) => Err(err),
        }
    }
    #[inline]
    pub fn success(&self) -> bool {
        match self {
            Verification::Success => true,
            _ => false,
        }
    }
    #[inline]
    pub fn failure(&self) -> bool {
        !self.success()
    }
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Error {
            kind: kind,
            cause: None,
        }
    }

    pub fn new_(kind: ErrorKind, cause: Box<dyn std::error::Error>) -> Self {
        Error {
            kind: kind,
            cause: Some(cause),
        }
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ErrorKind::Failure => write!(f, "The current state of the leader selection is invalid"),
            ErrorKind::NoLeaderForThisSlot => write!(f, "No leader available for this block date"),
            ErrorKind::IncompatibleBlockVersion => {
                write!(f, "The block Version is incompatible with LeaderSelection.")
            }
            ErrorKind::IncompatibleLeadershipMode => {
                write!(f, "Incompatible leadership mode (the proof is invalid)")
            }
            ErrorKind::InvalidLeader => write!(f, "Block has unexpected block leader"),
            ErrorKind::InvalidLeaderSignature => write!(f, "Block signature is invalid"),
            ErrorKind::InvalidBlockMessage => write!(f, "Invalid block message"),
            ErrorKind::InvalidStateUpdate => write!(f, "Invalid State Update"),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(cause) = &self.cause {
            write!(f, "{}: {}", self.kind, cause)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}
impl std::error::Error for Error {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.cause.as_ref().map(std::ops::Deref::deref)
    }
}
