use super::Error;
use crate::utils::vote;
use crate::utils::OutputFormat;
use chain_vote::EncryptedTally;
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::{PrivateTallyState, Tally};
use rayon::prelude::*;
use serde::Serialize;
use std::convert::TryInto;
use std::path::PathBuf;
use structopt::StructOpt;

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
