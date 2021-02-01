use super::Error;
use crate::jcli_app::utils::{io, OutputFormat};
use chain_vote::EncryptedTally;
use jormungandr_lib::interfaces::{PrivateTallyState, Tally};
use rayon::prelude::*;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct TallyDecryptWithAllShares {
    /// The path to hex-encoded encrypted tally state. If this parameter is not
    /// specified, the encrypted tally state will be read from the standard
    /// input.
    #[structopt(long = "tally")]
    encrypted_tally: Option<PathBuf>,
    /// The minimum number of shares needed for decryption
    #[structopt(long = "threshold", default_value = "3")]
    threshold: usize,
    /// Maximum supported number of votes
    #[structopt(long = "max-votes")]
    max_votes: u64,
    /// The path to encoded necessary shares. If this parameter is not
    /// specified, the shares will be read from the standard input.
    #[structopt(long = "shares")]
    shares: Option<PathBuf>,
    #[structopt(flatten)]
    output_format: OutputFormat,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct TallyVotePlanWithAllShares {
    /// The path to json-encoded vote plan to decrypt. If this parameter is not
    /// specified, the vote plan will be read from the standard
    /// input.
    #[structopt(long = "vote-plan-file")]
    vote_plan: Option<PathBuf>,
    /// The id of the vote plan to decrypt.
    /// Can be left unspecified if there is only one vote plan in the input
    #[structopt(long = "vote-plan-id")]
    vote_plan_id: Option<String>,
    /// The minimum number of shares needed for decryption
    #[structopt(long = "threshold", default_value = "3")]
    threshold: usize,
    /// The path to json-encoded necessary base64 shares. If this parameter is not
    /// specified, the shares will be read from the standard input.
    #[structopt(long = "shares")]
    shares: Option<PathBuf>,
    #[structopt(flatten)]
    output_format: OutputFormat,
}

#[derive(Serialize)]
struct Output {
    result: Vec<u64>,
}

fn read_shares_from_file<P: AsRef<Path>>(
    share_path: &Option<P>,
    threshold: usize,
    proposals: usize,
) -> Result<Vec<Vec<chain_vote::TallyDecryptShare>>, Error> {
    let shares: Vec<Vec<String>> = serde_json::from_reader(io::open_file_read(share_path)?)?;
    if shares[0].len() < threshold || shares.len() != proposals {
        return Err(Error::MissingShares);
    }
    shares
        .into_iter()
        .map(|v| {
            v.into_iter()
                .map(|share| {
                    chain_vote::TallyDecryptShare::from_bytes(&base64::decode(share)?)
                        .ok_or(Error::DecryptionShareRead)
                })
                .collect::<Result<Vec<_>, Error>>()
        })
        .collect::<Result<Vec<_>, Error>>()
}

impl TallyDecryptWithAllShares {
    pub fn exec(&self) -> Result<(), Error> {
        let encrypted_tally_hex = io::read_line(&self.encrypted_tally)?;
        let encrypted_tally_bytes = base64::decode(encrypted_tally_hex)?;
        let encrypted_tally =
            EncryptedTally::from_bytes(&encrypted_tally_bytes).ok_or(Error::EncryptedTallyRead)?;

        let shares = read_shares_from_file(&self.shares, self.threshold, 1)?;

        let state = encrypted_tally.state();
        let result = chain_vote::tally(
            self.max_votes,
            &state,
            &shares[0][..],
            &chain_vote::TallyOptimizationTable::generate_with_balance(self.max_votes, 1),
        )?;
        let output = self
            .output_format
            .format_json(serde_json::to_value(Output {
                result: result.votes,
            })?)?;

        println!("{}", output);

        Ok(())
    }
}

impl TallyVotePlanWithAllShares {
    pub fn exec(&self) -> Result<(), Error> {
        let mut vote_plan =
            super::get_vote_plan_by_id(&self.vote_plan, self.vote_plan_id.as_deref())?;
        let shares =
            read_shares_from_file(&self.shares, self.threshold, vote_plan.proposals.len())?;
        let mut max_stake = 0;
        let mut encrypted_tallies = Vec::new();
        for proposal in &mut vote_plan.proposals {
            match proposal.tally.take() {
                Some(Tally::Private {
                    state:
                        PrivateTallyState::Encrypted {
                            encrypted_tally,
                            total_stake,
                        },
                }) => {
                    max_stake = std::cmp::max(total_stake.into(), max_stake);
                    encrypted_tallies.push(
                        EncryptedTally::from_bytes(&encrypted_tally.into_bytes())
                            .ok_or(Error::EncryptedTallyRead)?,
                    );
                }
                other => return Err(Error::PrivateTallyExpected { found: other }),
            }
        }
        let table = chain_vote::TallyOptimizationTable::generate(max_stake);

        vote_plan.proposals = vote_plan
            .proposals
            .into_par_iter()
            .zip(encrypted_tallies.into_par_iter())
            .zip(shares.into_par_iter())
            .map(|((mut proposal, encrypted_tally), shares)| {
                let decrypted =
                    chain_vote::tally(max_stake, &encrypted_tally.state(), &shares, &table)?;
                proposal.tally = Some(Tally::Private {
                    state: PrivateTallyState::Decrypted {
                        result: decrypted.into(),
                    },
                });
                Ok(proposal)
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let output = self
            .output_format
            .format_json(serde_json::to_value(vote_plan)?)?;
        println!("{}", output);

        Ok(())
    }
}
