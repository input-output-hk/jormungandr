use super::Error;
use crate::jcli_app::utils::io;
use bech32::FromBase32;
use chain_vote::{EncryptedTally, OpeningVoteKey};
use std::path::PathBuf;
use structopt::StructOpt;

/// Create the decryption share for decrypting the tally of private voting.
/// The outputs are provided as hex-encoded byte sequences.
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct TallyGenerateDecryptionShare {
    /// The path to hex-encoded encrypted tally state. If this parameter is not
    /// specified, the encrypted tally state will be read from the standard
    /// input.
    #[structopt(long = "tally")]
    encrypted_tally: Option<PathBuf>,
    /// The path to hex-encoded decryption key.
    #[structopt(long = "key")]
    decryption_key: PathBuf,
}

impl TallyGenerateDecryptionShare {
    pub fn exec(&self) -> Result<(), Error> {
        let encrypted_tally_hex = io::read_line(&self.encrypted_tally)?;
        let encrypted_tally_bytes = base64::decode(encrypted_tally_hex)?;
        let encrypted_tally =
            EncryptedTally::from_bytes(&encrypted_tally_bytes).ok_or(Error::EncryptedTallyRead)?;

        let decryption_key = {
            let data = io::read_line(&Some(&self.decryption_key))?;
            bech32::decode(&data)
                .map_err(Error::from)
                .and_then(|(hrp, raw_key)| {
                    if hrp != crate::jcli_app::vote::bech32_constants::MEMBER_SK_HRP {
                        return Err(Error::InvalidSecretKey);
                    }
                    OpeningVoteKey::from_bytes(
                        &Vec::<u8>::from_base32(&raw_key).map_err(|_| Error::DecryptionKeyRead)?,
                    )
                    .ok_or(Error::DecryptionKeyRead)
                })?
        };

        let (_state, share) = encrypted_tally.finish(&decryption_key);
        println!("{}", base64::encode(share.to_bytes()));

        Ok(())
    }
}
