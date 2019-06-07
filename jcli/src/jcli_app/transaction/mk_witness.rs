use bech32::{Bech32, ToBase32 as _};
use chain_core::property::Serialize as _;
use chain_crypto::bech32::Bech32 as _;
use chain_crypto::{AsymmetricKey, SecretKey};
use chain_impl_mockchain::{
    account::SpendingCounter,
    block::HeaderHash,
    transaction::{TransactionId, Witness},
};
use jcli_app::transaction::{common, Error};
use jcli_app::utils::{error::CustomErrorFiller, io};
use std::{io::Write, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct MkWitness {
    /// the Transaction ID of the witness to sign
    #[structopt(name = "TRANSACTION_ID")]
    pub transaction_id: TransactionId,

    /// the file path to the file to write the witness in.
    /// If omitted it will be printed to the standard output.
    pub output: Option<PathBuf>,

    /// the type of witness to build: account, UTxO or Legacy UtxO
    #[structopt(long = "type", parse(try_from_str))]
    pub witness_type: WitnessType,

    /// the hash of the block0, the first block of the blockchain
    #[structopt(long = "genesis-block-hash", parse(try_from_str))]
    pub genesis_block_hash: HeaderHash,

    /// value is mandatory is `--type=account' It is the counter for
    /// every time the account is being utilized.
    #[structopt(long = "account-spending-counter")]
    pub account_spending_counter: Option<u32>,

    /// the file path to the file to read the signing key from.
    /// If omitted it will be read from the standard input.
    pub secret: Option<PathBuf>,
}

pub enum WitnessType {
    UTxO,
    OldUTxO,
    Account,
}

impl std::str::FromStr for WitnessType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "utxo" => Ok(WitnessType::UTxO),
            "legacy-utxo" => Ok(WitnessType::OldUTxO),
            "account" => Ok(WitnessType::Account),
            _ => Err("Invalid witness type, expected `utxo', `legacy-utxo' or `account'"),
        }
    }
}

impl MkWitness {
    fn secret<A: AsymmetricKey>(&self) -> Result<SecretKey<A>, Error> {
        let bech32_str =
            common::read_line(&self.secret).map_err(|source| Error::SecretFileReadFailed {
                source,
                path: common::path_to_path_buf(&self.secret),
            })?;
        SecretKey::try_from_bech32_str(&bech32_str).map_err(|source| Error::SecretFileMalformed {
            source,
            path: common::path_to_path_buf(&self.secret),
        })
    }

    pub fn exec(self) -> Result<(), Error> {
        let witness = match self.witness_type {
            WitnessType::UTxO => {
                let secret_key = self.secret()?;
                Witness::new_utxo(&self.genesis_block_hash, &self.transaction_id, &secret_key)
            }
            // TODO unimplemented!()
            WitnessType::OldUTxO => return Err(Error::MakeWitnessLegacyUtxoUnsupported)?,
            WitnessType::Account => {
                let account_spending_counter = self
                    .account_spending_counter
                    .ok_or(Error::MakeWitnessAccountCounterMissing)
                    .map(SpendingCounter::from)?;

                let secret_key = self.secret()?;
                Witness::new_account(
                    &self.genesis_block_hash,
                    &self.transaction_id,
                    &account_spending_counter,
                    &secret_key,
                )
            }
        };

        self.write_witness(&witness)
    }

    fn write_witness(&self, witness: &Witness) -> Result<(), Error> {
        let mut writer =
            io::open_file_write(&self.output).map_err(|source| Error::WitnessFileWriteFailed {
                source,
                path: self.output.clone().unwrap_or_default(),
            })?;
        let bytes =
            witness
                .serialize_as_vec()
                .map_err(|source| Error::WitnessFileSerializationFailed {
                    source,
                    filler: CustomErrorFiller,
                })?;

        let base32 = bytes.to_base32();
        let bech32 = Bech32::new("witness".to_owned(), base32)?;
        writeln!(writer, "{}", bech32).map_err(|source| Error::WitnessFileWriteFailed {
            source,
            path: self.output.clone().unwrap_or_default(),
        })
    }
}
