mod decrypt_tally;
mod decryption_shares;

use super::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Tally {
    /// Create a decryption share for private voting tally.
    ///
    /// The decryption share data will be printed in hexadecimal encoding
    /// on standard output.
    DecryptionShares(decryption_shares::TallyGenerateVotePlanDecryptionShares),
    /// Merge multiple sets of shares in a single object to be used in the
    /// decryption of a vote plan.
    MergeShares(decryption_shares::MergeShares),
    /// Decrypt all proposals in a vote plan.
    ///
    /// The decrypted tally data will be printed in hexadecimal encoding
    /// on standard output.
    DecryptResults(decrypt_tally::TallyVotePlanWithAllShares),
}

impl Tally {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Tally::DecryptionShares(cmd) => cmd.exec(),
            Tally::DecryptResults(cmd) => cmd.exec(),
            Tally::MergeShares(cmd) => cmd.exec(),
        }
    }
}
