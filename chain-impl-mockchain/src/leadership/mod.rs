use crate::key::{deserialize_public_key, serialize_public_key, Hash};
use chain_core::property;
use chain_crypto::algorithms::vrf::vrf::{self, ProvenOutputSeed};
use chain_crypto::{Curve25519_2HashDH, Ed25519Extended, FakeMMM, PublicKey, SecretKey};

pub mod bft;
// pub mod genesis;
pub mod none;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BftLeader(pub(crate) PublicKey<Ed25519Extended>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenesisPraosId(Hash);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenesisPraosLeader {
    pub(crate) kes_public_key: PublicKey<FakeMMM>,
    pub(crate) vrf_public_key: PublicKey<Curve25519_2HashDH>,
}

impl GenesisPraosLeader {
    pub fn get_id(&self) -> GenesisPraosId {
        unimplemented!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorKind {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LeaderId {
    None,
    Bft(bft::LeaderId),
    GenesisPraos(GenesisPraosLeader),
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

impl chain_core::property::LeaderId for LeaderId {}

impl Verification {
    pub fn success() -> Self {
        Verification::Success
    }
    pub fn failed(error: Error) -> Self {
        Verification::Failure(error)
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
