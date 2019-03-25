use crate::{
    block::{BlockVersion, BlockVersionTag, Header},
    state::State,
};
use chain_crypto::algorithms::vrf::vrf::ProvenOutputSeed;
use chain_crypto::{Curve25519_2HashDH, Ed25519Extended, FakeMMM, SecretKey};

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
    None(none::NoLeadership),
    Bft(bft::BftLeaderSelection),
    GenesisPraos(genesis::GenesisLeaderSelection),
}

pub struct Leadership {
    inner: Inner,
}

impl Inner {
    #[inline]
    fn verify_version(&self, block_version: &BlockVersion) -> Verification {
        match self {
            Inner::None(_)
                if block_version == &BlockVersionTag::ConsensusNone.to_block_version() =>
            {
                Verification::Success
            }
            Inner::Bft(_) if block_version == &BlockVersionTag::ConsensusBft.to_block_version() => {
                Verification::Success
            }
            _ => Verification::Failure(Error::new(ErrorKind::IncompatibleBlockVersion)),
        }
    }

    #[inline]
    fn verify_leader(&self, block_header: &Header) -> Verification {
        match self {
            Inner::None(none) => none.verify(block_header),
            Inner::Bft(bft) => bft.verify(block_header),
            Inner::GenesisPraos(genesis_praos) => genesis_praos.verify(block_header),
        }
    }
}

impl Leadership {
    pub fn new(state: &State) -> Self {
        match BlockVersionTag::from_block_version(state.settings.block_version.clone()) {
            Some(BlockVersionTag::ConsensusNone) => Leadership {
                inner: Inner::None(none::NoLeadership),
            },
            Some(BlockVersionTag::ConsensusBft) => Leadership {
                inner: Inner::Bft(bft::BftLeaderSelection::new(state).unwrap()),
            },
            Some(BlockVersionTag::ConsensusGenesisPraos) => Leadership {
                inner: Inner::GenesisPraos(genesis::GenesisLeaderSelection::new(state)),
            },
            None => unimplemented!(),
        }
    }

    pub fn verify(&self, block_header: &Header) -> Verification {
        try_check!(self.inner.verify_version(block_header.block_version()));

        try_check!(self.inner.verify_leader(block_header));
        Verification::Success
    }
}

impl LeaderSelection for Leadership {
    type Error = Error;
    type LeaderId = LeaderId;
    type Block = Block;
    type State = State;

    fn retrieve(state: &Self::State) -> Self {
        Self::new(state)
    }

    /// return the ID of the leader of the blockchain at the given
    /// date.
    fn get_leader_at(
        &self,
        date: <Self::Block as property::Block>::Date,
    ) -> Result<Self::LeaderId, Self::Error> {
        match &self.inner {
            Inner::None(none) => none.get_leader_at(date),
            Inner::Bft(bft) => bft.get_leader_at(date).map(LeaderId::Bft),
            Inner::GenesisPraos(genesis_praos) => genesis_praos
                .get_leader_at(date)
                .map(LeaderId::GenesisPraos),
        }
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
