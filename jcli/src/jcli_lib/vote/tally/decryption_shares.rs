use super::Error;
use crate::jcli_lib::utils::{
    io,
    vote::{self, MemberVotePlanShares, VotePlanDecryptShares},
};
use chain_crypto::bech32::Bech32;
use chain_vote::tally::{EncryptedTally, OpeningVoteKey};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{PrivateTallyState, Tally},
};
use std::{convert::TryFrom, path::PathBuf};
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
    /// The path to bech32-encoded decryption key.
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

impl TallyGenerateVotePlanDecryptionShares {
    pub fn exec(&self) -> Result<(), Error> {
        let vote_plan =
            vote::get_vote_plan_by_id(self.vote_plan.as_ref(), self.vote_plan_id.as_ref())?;
        let line = io::read_line(&Some(&self.key))?;
        let decryption_key = OpeningVoteKey::try_from_bech32_str(&line)?;

        let shares = vote_plan
            .proposals
            .into_iter()
            .filter_map(|prop| match prop.tally {
                Tally::Private {
                    state:
                        PrivateTallyState::Encrypted {
                            encrypted_tally, ..
                        },
                } => {
                    let encrypted_tally =
                        EncryptedTally::from_bytes(&encrypted_tally.into_bytes())?;
                    Some(encrypted_tally.partial_decrypt(&mut rand::thread_rng(), &decryption_key))
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
