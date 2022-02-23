use super::Error;
use crate::jcli_lib::utils::{
    vote::{self, SharesError},
    OutputFormat,
};
use chain_vote::tally::{batch_decrypt, EncryptedTally};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{PrivateTallyState, Tally},
};
use rayon::prelude::*;
use serde::Serialize;
use std::{convert::TryInto, path::PathBuf};
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

        let committee_member_keys = vote_plan.committee_member_keys.clone();

        let validated_tallies = encrypted_tallies
            .into_par_iter()
            .zip(shares.into_par_iter())
            .map(|(encrypted_tally, shares)| {
                let encrypted_tally = EncryptedTally::from_bytes(&encrypted_tally)
                    .ok_or(Error::EncryptedTallyRead)?;
                encrypted_tally
                    .validate_partial_decryptions(&committee_member_keys, &shares)
                    .map_err(SharesError::ValidationFailed)
                    .map_err(Error::SharesError)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let decrypted_tallies = batch_decrypt(validated_tallies)?;

        for (proposal, decrypted_tally) in vote_plan
            .proposals
            .iter_mut()
            .zip(decrypted_tallies.into_iter())
        {
            proposal.tally = Some(Tally::Private {
                state: PrivateTallyState::Decrypted {
                    result: decrypted_tally.into(),
                },
            })
        }

        let output = self
            .output_format
            .format_json(serde_json::to_value(vote_plan)?)?;
        println!("{}", output);

        Ok(())
    }
}
