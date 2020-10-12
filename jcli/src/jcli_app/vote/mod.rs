use structopt::StructOpt;
use thiserror::Error;

mod decryption_share;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("failed to read hex bytes")]
    Hex(#[from] hex::FromHexError),
    #[error("failed to read encrypted tally bytes")]
    EncryptedTallyRead,
    #[error("failed to read decryption key bytes")]
    DecryptionKeyRead,
    #[error(transparent)]
    FormatError(#[from] crate::jcli_app::utils::output_format::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Vote {
    /// Create decryption share for private voting tally.
    NewTallyDecryptionShare(decryption_share::TallyDecryptionShare),
}

impl Vote {
    pub fn exec(&self) -> Result<(), Error> {
        match self {
            Vote::NewTallyDecryptionShare(cmd) => cmd.exec(),
        }
    }
}
