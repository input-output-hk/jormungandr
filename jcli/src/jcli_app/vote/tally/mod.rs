mod decrypt_shares;
mod decryption_tally;

use super::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Tally {
    /// Create a decryption share for private voting tally.
    ///
    /// The decryption share data will be printed in hexadecimal encoding
    /// on standard output.
    DecryptionShare(decryption_tally::TallyGenerateDecryptionShare),
    /// Create a decryption share for private voting tally.
    ///
    /// The decryption share data will be printed in hexadecimal encoding
    /// on standard output.
    VotePlanDecryptionShares(decryption_tally::TallyGenerateVotePlanDecryptionShares),
    /// Merge multiple sets of shares in a single object to be used in the
    /// decryption of a vote plan.
    MergeShares(decryption_tally::MergeShares),
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
            Tally::VotePlanDecryptionShares(cmd) => cmd.exec(),
            Tally::Decrypt(cmd) => cmd.exec(),
            Tally::DecryptVotePlan(cmd) => cmd.exec(),
            Tally::MergeShares(cmd) => cmd.exec(),
        }
    }
}
