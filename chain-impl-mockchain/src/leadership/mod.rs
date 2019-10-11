use crate::{
    block::{BlockDate, BlockVersion, ConsensusVersion, Header},
    certificate::PoolId,
    date::Epoch,
    ledger::{Ledger, LedgerParameters},
};
use chain_crypto::{Curve25519_2HashDH, Ed25519, SecretKey, SumEd25519_12};
use chain_time::era::TimeEra;

pub mod bft;
pub mod genesis;

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

pub struct BftLeader {
    pub sig_key: SecretKey<Ed25519>,
}

pub struct GenesisLeader {
    pub node_id: PoolId,
    pub sig_key: SecretKey<SumEd25519_12>,
    pub vrf_key: SecretKey<Curve25519_2HashDH>,
}

pub struct Leader {
    pub bft_leader: Option<BftLeader>,
    pub genesis_leader: Option<GenesisLeader>,
}

pub enum LeaderOutput {
    None,
    Bft(bft::LeaderId),
    GenesisPraos(genesis::Witness),
}

pub enum LeadershipConsensus {
    Bft(bft::LeadershipData),
    GenesisPraos(genesis::LeadershipData),
}

/// Leadership represent a given epoch and their associated leader or metadata.
pub struct Leadership {
    // Specific epoch where the leadership apply
    epoch: Epoch,
    // Give the closest parameters associated with date keeping given a leadership
    era: TimeEra,
    // Consensus specific metadata required for verifying/evaluating leaders
    inner: LeadershipConsensus,
    // Ledger evaluation parameters fixed for a given epoch
    ledger_parameters: LedgerParameters,
}

impl LeadershipConsensus {
    #[inline]
    fn verify_version(&self, block_version: BlockVersion) -> Verification {
        match self {
            LeadershipConsensus::Bft(_) if block_version == BlockVersion::Ed25519Signed => {
                Verification::Success
            }
            LeadershipConsensus::GenesisPraos(_) if block_version == BlockVersion::KesVrfproof => {
                Verification::Success
            }
            _ => Verification::Failure(Error::new(ErrorKind::IncompatibleBlockVersion)),
        }
    }

    #[inline]
    fn verify_leader(&self, block_header: &Header) -> Verification {
        match self {
            LeadershipConsensus::Bft(bft) => bft.verify(block_header),
            LeadershipConsensus::GenesisPraos(genesis_praos) => genesis_praos.verify(block_header),
        }
    }

    #[inline]
    fn is_leader(&self, leader: &Leader, date: BlockDate) -> Result<LeaderOutput, Error> {
        match self {
            LeadershipConsensus::Bft(bft) => match leader.bft_leader {
                Some(ref bft_leader) => {
                    let bft_leader_id = bft.get_leader_at(date)?;
                    if bft_leader_id == bft_leader.sig_key.to_public().into() {
                        Ok(LeaderOutput::Bft(bft_leader_id))
                    } else {
                        Ok(LeaderOutput::None)
                    }
                }
                None => Ok(LeaderOutput::None),
            },
            LeadershipConsensus::GenesisPraos(genesis_praos) => match leader.genesis_leader {
                None => Ok(LeaderOutput::None),
                Some(ref gen_leader) => {
                    match genesis_praos.leader(&gen_leader.node_id, &gen_leader.vrf_key, date) {
                        Ok(Some(witness)) => Ok(LeaderOutput::GenesisPraos(witness)),
                        _ => Ok(LeaderOutput::None),
                    }
                }
            },
        }
    }
}

impl Leadership {
    pub fn new(epoch: Epoch, ledger: &Ledger) -> Self {
        let inner = match ledger.settings.consensus_version {
            ConsensusVersion::Bft => {
                LeadershipConsensus::Bft(bft::LeadershipData::new(ledger).unwrap())
            }
            ConsensusVersion::GenesisPraos => {
                LeadershipConsensus::GenesisPraos(genesis::LeadershipData::new(epoch, ledger))
            }
        };
        Leadership {
            epoch: epoch,
            era: ledger.era.clone(),
            inner,
            ledger_parameters: ledger.get_ledger_parameters(),
        }
    }

    /// get the epoch associated to the `Leadership`
    #[inline]
    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    /// Create a Block date given a leadership and a relative epoch slot
    ///
    /// # Panics
    ///
    /// If the slot index is not valid given the leadership, out of bound date
    pub fn date_at_slot(&self, slot_id: u32) -> BlockDate {
        assert!(slot_id < self.era.slots_per_epoch());
        BlockDate {
            epoch: self.epoch(),
            slot_id: slot_id,
        }
    }

    /// get the TimeEra associated to the `Leadership`
    #[inline]
    pub fn era(&self) -> &TimeEra {
        &self.era
    }

    /// get the consensus associated with the `Leadership`
    pub fn consensus(&self) -> &LeadershipConsensus {
        &self.inner
    }

    /// access the ledger parameter for the current leadership
    #[inline]
    pub fn ledger_parameters(&self) -> &LedgerParameters {
        &self.ledger_parameters
    }

    /// Verify whether this header has been produced by a leader that fits with the leadership
    ///
    pub fn verify(&self, block_header: &Header) -> Verification {
        try_check!(self.inner.verify_version(block_header.block_version()));

        try_check!(self.inner.verify_leader(block_header));
        Verification::Success
    }

    /// Test that the given leader object is able to create a valid block for the leadership
    /// at a given date.
    pub fn is_leader_for_date<'a>(
        &self,
        leader: &'a Leader,
        date: BlockDate,
    ) -> Result<LeaderOutput, Error> {
        self.inner.is_leader(leader, date)
    }
}

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

    pub fn new_<E>(kind: ErrorKind, cause: E) -> Self
    where
        E: std::error::Error + 'static,
    {
        Error {
            kind: kind,
            cause: Some(Box::new(cause)),
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
