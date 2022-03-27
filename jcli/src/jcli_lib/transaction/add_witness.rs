use crate::jcli_lib::{
    transaction::{common, Error},
    utils::io,
};
use bech32::{self, FromBase32 as _};
use chain_core::{packer::Codec, property::DeserializeFromSlice};
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
        const HRP: &str = "witness";

        let bech32_str =
            io::read_line(&Some(&self.witness)).map_err(|source| Error::WitnessFileReadFailed {
                source,
                path: self.witness.clone(),
            })?;

        let (hrp, data, _variant) = bech32::decode(bech32_str.trim()).map_err(|source| {
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
        Witness::deserialize_from_slice(&mut Codec::new(bytes.as_slice())).map_err(|source| {
            Error::WitnessFileDeserializationFailed {
                source,
                path: self.witness.clone(),
            }
        })
    }
}
