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
    /// Decrypt a tally with decryption shares.
    ///
    /// The decrypted tally data will be printed in hexadecimal encoding
    /// on standard output.
    Decrypt(decrypt_shares::TallyDecryptWithAllShares),
}

impl Tally {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Tally::DecryptionShare(cmd) => cmd.exec(),
            Tally::Decrypt(cmd) => cmd.exec(),
        }
    }
}
