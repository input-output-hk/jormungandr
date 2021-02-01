mod decrypt_shares;
mod decryption_tally;

use super::Error;
use crate::jcli_app::utils::io;
use jormungandr_lib::interfaces::VotePlanStatus;
use std::path::Path;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Tally {
    /// Create a decryption share for private voting tally.
    ///
    /// The decryption share data will be printed in hexadecimal encoding
    /// on standard output.
    DecryptionShare(decryption_tally::TallyGenerateDecryptionShare),
    /// Decrypt a tally with decryption shares.
    ///
    /// The decrypted tally data will be printed in hexadecimal encoding
    /// on standard output.
    Decrypt(decrypt_shares::TallyDecryptWithAllShares),
    /// Decrypt all proposals in a vote plan.
    ///
    /// The decrypted tally data will be printed in hexadecimal encoding
    /// on standard output.
    DecryptVotePlan(decrypt_shares::TallyVotePlanWithAllShares),
}

impl Tally {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Tally::DecryptionShare(cmd) => cmd.exec(),
            Tally::Decrypt(cmd) => cmd.exec(),
            Tally::DecryptVotePlan(cmd) => cmd.exec(),
        }
    }
}

// Read json-encoded vote plan(s) from file and returns the one
// with the specified id. If there is only one vote plan in the input
// the id can be omitted
fn get_vote_plan_by_id<P: AsRef<Path>>(
    vote_plan_file: &Option<P>,
    id: Option<&str>,
) -> Result<VotePlanStatus, Error> {
    let mut vote_plans: Vec<VotePlanStatus> =
        serde_json::from_reader(io::open_file_read(vote_plan_file)?)
            .map_err(|_| Error::VotePlansRead)?;
    match id {
        Some(id) => vote_plans
            .into_iter()
            .find(|plan| plan.id.to_hex().contains(id))
            .ok_or(Error::VotePlanIdNotFound),
        None => {
            if vote_plans.len() != 1 {
                return Err(Error::UnclearVotePlan);
            }
            Ok(vote_plans.pop().unwrap())
        }
    }
}
