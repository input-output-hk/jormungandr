mod decrypt_shares;
mod decryption_tally;

use super::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Tally {
    /// Create decryption share for private voting tally.
    TallyDecryptionShare(decryption_tally::TallyGenerateDecryptionShare),
    /// Decrypt a tally with shares
    TallyDecryptWithShares(decrypt_shares::TallyDecryptWithAllShares),
}

impl Tally {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Tally::TallyDecryptionShare(cmd) => cmd.exec(),
            Tally::TallyDecryptWithShares(cmd) => cmd.exec(),
        }
    }
}
