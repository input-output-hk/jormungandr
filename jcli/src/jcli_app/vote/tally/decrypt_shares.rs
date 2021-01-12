use super::Error;
use crate::jcli_app::utils::vote::{self, SharesError};
use crate::jcli_app::utils::{io, OutputFormat};
use chain_vote::EncryptedTally;
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::{PrivateTallyState, TallyResult};
use rayon::prelude::*;
use std::convert::TryInto;
use std::io::BufRead;
use std::path::PathBuf;
use structopt::StructOpt;

// TODO: this decrypts a single proposal, we might remove it later
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
    /// Supported limit of vote stake (Ex: 10 votes with 10 stake power each would be 100 limit stake)
    #[structopt(long = "vote-stake-limit")]
    vote_stake_limit: u64,
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
    #[structopt(long)]
    vote_plan: Option<PathBuf>,
    /// The id of the vote plan to decrypt.
    /// Can be left unspecified if there is only one vote plan in the input
    #[structopt(long)]
    vote_plan_id: Option<Hash>,
    /// The minimum number of shares needed for decryption
    #[structopt(long, default_value = "3")]
    threshold: usize,
    /// The path to a JSON file containing decryption shares necessary to decrypt
    /// the vote plan. If this parameter is not specified, the shares will be read
    /// from the standard input.
    #[structopt(long)]
    shares: Option<PathBuf>,
    #[structopt(flatten)]
    output_format: OutputFormat,
}

#[derive(Serialize)]
struct Output {
    result: Vec<u64>,
}

impl TallyDecryptWithAllShares {
    pub fn exec(&self) -> Result<(), Error> {
        let encrypted_tally_hex = io::read_line(&self.encrypted_tally)?;
        let encrypted_tally_bytes = base64::decode(encrypted_tally_hex)?;
        let encrypted_tally =
            EncryptedTally::from_bytes(&encrypted_tally_bytes).ok_or(Error::EncryptedTallyRead)?;

        let mut shares_file = io::open_file_read(&self.shares)?;

        let shares: Vec<chain_vote::TallyDecryptShare> = {
            let mut shares = Vec::with_capacity(self.threshold);
            for _ in 0..self.threshold {
                let mut buff = String::new();
                shares_file.read_line(&mut buff)?;
                let buff = buff.trim_end();
                shares.push(
                    chain_vote::TallyDecryptShare::from_bytes(&base64::decode(buff)?)
                        .ok_or(SharesError::InvalidBinaryShare)?,
                );
            }
            shares
        };
        let state = encrypted_tally.state();
        let result = chain_vote::tally(
            self.vote_stake_limit,
            &state,
            &shares,
            &chain_vote::TallyOptimizationTable::generate_with_balance(self.max_votes, 1),
        )?;
        let output = self
            .output_format
            .format_json(serde_json::to_value(result)?)?;

        println!("{}", output);

        Ok(())
    }
}

impl TallyVotePlanWithAllShares {
    pub fn exec(&self) -> Result<(), Error> {
        let mut vote_plan =
            vote::get_vote_plan_by_id(self.vote_plan.as_ref(), self.vote_plan_id.as_ref())?;
        let shares: Vec<Vec<chain_vote::TallyDecryptShare>> =
            vote::read_vote_plan_shares_from_file(
                self.shares.as_ref(),
                vote_plan.proposals.len(),
                Some(self.threshold),
            )?
            .try_into()?;
        let mut max_stake = 0;
        let mut encrypted_tallies = Vec::new();
        // We need a first iteration to get the max stake used, and since we're there
        // we unwrap and check tallies as well
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
                    encrypted_tallies.push(encrypted_tally.into_bytes());
                }
                other => {
                    let found = match other {
                        Some(Tally::Public { .. }) => "public tally",
                        Some(Tally::Private { .. }) => "private decrypted tally",
                        None => "none",
                    };
                    return Err(Error::PrivateTallyExpected { found });
                }
            }
        }
        let table = chain_vote::TallyOptimizationTable::generate(max_stake);

        vote_plan.proposals = vote_plan
            .proposals
            .into_par_iter()
            .zip(encrypted_tallies.into_par_iter())
            .zip(shares.into_par_iter())
            .map(|((mut proposal, encrypted_tally), shares)| {
                let state = EncryptedTally::from_bytes(&encrypted_tally)
                    .ok_or(Error::EncryptedTallyRead)?
                    .state();
                let decrypted = chain_vote::tally(max_stake, &state, &shares, &table)?;
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
