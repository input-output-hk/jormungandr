use crate::{
    transaction::{common, Error},
    utils::io,
};
use bech32::{self, FromBase32 as _};
use chain_core::mempack::{ReadBuf, Readable as _};
use chain_impl_mockchain::transaction::Witness;
use std::path::PathBuf;
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub struct AddWitness {
    #[cfg_attr(feature = "structopt", structopt(flatten))]
    pub common: common::CommonTransaction,

    pub witness: PathBuf,
}

impl AddWitness {
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;

        let witness = self.witness()?;

        transaction.add_witness(witness)?;

        self.common.store(&transaction)?;
        Ok(())
    }

    fn witness(&self) -> Result<Witness, Error> {
        const HRP: &str = "witness";

        let bech32_str =
            io::read_line(&Some(&self.witness)).map_err(|source| Error::WitnessFileReadFailed {
                source,
                path: self.witness.clone(),
            })?;

        let (hrp, data) = bech32::decode(bech32_str.trim()).map_err(|source| {
            Error::WitnessFileBech32Malformed {
                source,
                path: self.witness.clone(),
            }
        })?;
        if hrp != HRP {
            return Err(Error::WitnessFileBech32HrpInvalid {
                expected: HRP,
                actual: hrp,
                path: self.witness.clone(),
            });
        }
        let bytes =
            Vec::from_base32(&data).map_err(|source| Error::WitnessFileBech32Malformed {
                source,
                path: self.witness.clone(),
            })?;
        Witness::read(&mut ReadBuf::from(&bytes)).map_err(|source| {
            Error::WitnessFileDeserializationFailed {
                source,
                path: self.witness.clone(),
            }
        })
    }
}
