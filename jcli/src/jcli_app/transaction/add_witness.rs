use crate::jcli_app::transaction::{common, Error};
use crate::jcli_app::utils::io;
use bech32::{Bech32, FromBase32 as _};
use chain_core::mempack::{ReadBuf, Readable as _};
use chain_impl_mockchain::transaction::Witness;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AddWitness {
    #[structopt(flatten)]
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
        const HRP: &'static str = "witness";

        let bech32_str =
            io::read_line(&Some(&self.witness)).map_err(|source| Error::WitnessFileReadFailed {
                source,
                path: self.witness.clone(),
            })?;

        let bech32: Bech32 =
            bech32_str
                .trim()
                .parse()
                .map_err(|source| Error::WitnessFileBech32Malformed {
                    source,
                    path: self.witness.clone(),
                })?;
        if bech32.hrp() != HRP {
            return Err(Error::WitnessFileBech32HrpInvalid {
                expected: HRP,
                actual: bech32.hrp().to_string(),
                path: self.witness.clone(),
            });
        }
        let bytes = Vec::from_base32(bech32.data()).map_err(|source| {
            Error::WitnessFileBech32Malformed {
                source,
                path: self.witness.clone(),
            }
        })?;
        Witness::read(&mut ReadBuf::from(&bytes)).map_err(|source| {
            Error::WitnessFileDeserializationFailed {
                source,
                path: self.witness.clone(),
            }
        })
    }
}
