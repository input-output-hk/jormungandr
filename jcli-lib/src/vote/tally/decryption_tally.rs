use super::Error;
use crate::jcli_lib::utils::io;
use crate::jcli_lib::utils::vote::{self, MemberVotePlanShares, VotePlanDecryptShares};
use bech32::FromBase32;
use chain_vote::{EncryptedTally, OpeningVoteKey};
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::{PrivateTallyState, Tally};
use std::convert::TryFrom;
use std::path::Path;
use std::path::PathBuf;
use structopt::StructOpt;

/// Create decryption shares for all proposals in a vote plan.
///
/// The decryption share data will be printed in hexadecimal encoding
/// on standard output.
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct TallyGenerateVotePlanDecryptionShares {
    /// The path to json-encoded vote plan to decrypt. If this parameter is not
    /// specified, the vote plan will be read from standard input.
    #[structopt(long)]
    vote_plan: Option<PathBuf>,
    /// The id of the vote plan to decrypt.
    /// Can be left unspecified if there is only one vote plan in the input
    #[structopt(long)]
    vote_plan_id: Option<Hash>,
    /// The path to hex-encoded decryption key.
    #[structopt(long)]
    key: PathBuf,
}

/// Merge multiple sets of shares in a single object to be used in the
/// decryption of a vote plan.
///
/// The data will be printed in hexadecimal encoding
/// on standard output.
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct MergeShares {
    /// The path to the shares to merge
    shares: Vec<PathBuf>,
}

fn read_decryption_key<P: AsRef<Path>>(path: &Option<P>) -> Result<OpeningVoteKey, Error> {
    let data = io::read_line(path)?;
    bech32::decode(&data)
        .map_err(Error::from)
        .and_then(|(hrp, raw_key)| {
            if hrp != crate::jcli_lib::vote::bech32_constants::MEMBER_SK_HRP {
                return Err(Error::InvalidSecretKey);
            }
            OpeningVoteKey::from_bytes(
                &Vec::<u8>::from_base32(&raw_key).map_err(|_| Error::DecryptionKeyRead)?,
            )
            .ok_or(Error::DecryptionKeyRead)
        })
}

impl TallyGenerateVotePlanDecryptionShares {
    pub fn exec(&self) -> Result<(), Error> {
        let vote_plan =
            vote::get_vote_plan_by_id(self.vote_plan.as_ref(), self.vote_plan_id.as_ref())?;
        let decryption_key = read_decryption_key(&Some(&self.key))?;

        let shares = vote_plan
            .proposals
            .into_iter()
            .filter_map(|prop| match prop.tally {
                Some(Tally::Private {
                    state:
                        PrivateTallyState::Encrypted {
                            encrypted_tally, ..
                        },
                }) => {
                    let encrypted_tally =
                        EncryptedTally::from_bytes(&encrypted_tally.into_bytes())?;
                    Some(encrypted_tally.finish(&decryption_key).1)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        println!(
            "{}",
            serde_json::to_value(MemberVotePlanShares::from(shares))?
        );
        Ok(())
    }
}

impl MergeShares {
    pub fn exec(&self) -> Result<(), Error> {
        let shares = self
            .shares
            .iter()
            .map(|path| Ok(serde_json::from_reader(io::open_file_read(&Some(path))?)?))
            .collect::<Result<Vec<MemberVotePlanShares>, Error>>()?;
        let vote_plan_shares = VotePlanDecryptShares::try_from(shares)?;
        println!("{}", serde_json::to_string(&vote_plan_shares)?);
        Ok(())
    }
}
