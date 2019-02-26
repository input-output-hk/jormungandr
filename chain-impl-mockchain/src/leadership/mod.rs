use crate::block::SignedBlock;
use crate::key::PublicKey;
use crate::update::LeaderSelectionDiff;
use chain_core::property;

pub mod bft;

#[derive(Debug)]
pub enum LeaderSelection {
    BFT(bft::BftLeaderSelection<PublicKey>),
    Genesis,
}

impl property::LeaderSelection for LeaderSelection {
    type Update = LeaderSelectionDiff;
    type Block = SignedBlock;
    type Error = Error;
    type LeaderId = PublicKey;

    fn diff(&self, input: &Self::Block) -> Result<Self::Update, Self::Error> {
        let mut update = <Self::Update as property::Update>::empty();

        match self {
            LeaderSelection::BFT(ref bft) => {
                update.bft = property::LeaderSelection::diff(bft, input).map_err(Error::Bft)?;
            }
            LeaderSelection::Genesis => unimplemented!(),
        }

        Ok(update)
    }
    fn apply(&mut self, update: Self::Update) -> Result<(), Self::Error> {
        match self {
            LeaderSelection::BFT(ref mut bft) => {
                property::LeaderSelection::apply(bft, update.bft).map_err(Error::Bft)?;
            }
            LeaderSelection::Genesis => unimplemented!(),
        }
        Ok(())
    }

    #[inline]
    fn get_leader_at(
        &self,
        date: <Self::Block as property::Block>::Date,
    ) -> Result<Self::LeaderId, Self::Error> {
        match self {
            LeaderSelection::BFT(ref bft) => {
                property::LeaderSelection::get_leader_at(bft, date).map_err(Error::Bft)
            }
            LeaderSelection::Genesis => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Bft(bft::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Bft(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
