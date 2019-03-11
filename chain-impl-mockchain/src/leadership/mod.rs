use crate::key::{deserialize_public_key, serialize_public_key};
use crate::stake::StakePoolId;
use chain_core::property;
use chain_crypto::algorithms::vrf::vrf::{self, ProvenOutputSeed};
use chain_crypto::{Ed25519Extended, FakeMMM, PublicKey, SecretKey};

pub mod bft;
pub mod genesis;
pub mod none;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BftLeader(pub(crate) PublicKey<Ed25519Extended>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenesisPraosLeader {
    pub(crate) kes_public_key: PublicKey<FakeMMM>,
    pub(crate) vrf_public_key: vrf::PublicKey,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PublicLeader {
    None,
    Bft(BftLeader),
    GenesisPraos(GenesisPraosLeader),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub(crate) previous_leader: PublicLeader,
    pub(crate) next_leader: PublicLeader,
}

pub enum Leader {
    None,
    BftLeader(SecretKey<Ed25519Extended>),
    GenesisPraos(SecretKey<FakeMMM>, vrf::SecretKey, ProvenOutputSeed),
}

impl chain_core::property::LeaderId for BftLeader {}
impl chain_core::property::LeaderId for PublicLeader {}

impl From<PublicKey<Ed25519Extended>> for BftLeader {
    fn from(key: PublicKey<Ed25519Extended>) -> Self {
        BftLeader(key)
    }
}

impl property::Update for Update {
    fn empty() -> Self {
        Update {
            previous_leader: PublicLeader::None,
            next_leader: PublicLeader::None,
        }
    }
    fn union(&mut self, other: Self) -> &mut Self {
        self.next_leader = other.next_leader;
        self
    }
    fn inverse(mut self) -> Self {
        std::mem::swap(&mut self.previous_leader, &mut self.next_leader);
        self
    }
}

impl property::Serialize for BftLeader {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        serialize_public_key(&self.0, writer)
    }
}

impl property::Deserialize for BftLeader {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        deserialize_public_key(reader).map(BftLeader)
    }
}

impl AsRef<PublicKey<Ed25519Extended>> for BftLeader {
    fn as_ref(&self) -> &PublicKey<Ed25519Extended> {
        &self.0
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ErrorKind::IncompatibleBlockVersion => {
                write!(f, "The block Version is incompatible with LeaderSelection.")
            }
            ErrorKind::IncompatibleLeadershipMode => write!(f, "Incompatible leadership mode"),
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

#[cfg(test)]
pub mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for BftLeader {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand_chacha::ChaChaRng;
            use rand_core::SeedableRng;
            let mut seed = [0; 32];
            for byte in seed.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            let mut rng = ChaChaRng::from_seed(seed);
            BftLeader(SecretKey::generate(&mut rng).to_public())
        }
    }
}
