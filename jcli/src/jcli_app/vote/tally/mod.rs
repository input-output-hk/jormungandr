mod decrypt_shares;
mod decryption_tally;

use super::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Tally {
    /// Create decryption share for private voting tally.
    GenerateDecryptionShare(decryption_tally::TallyGenerateDecryptionShare),
    /// Decrypt a tally with shares
    DecryptWithShares(decrypt_shares::TallyDecryptWithAllShares),
}

impl Tally {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Tally::GenerateDecryptionShare(cmd) => cmd.exec(),
            Tally::DecryptWithShares(cmd) => cmd.exec(),
        }
    }
}
