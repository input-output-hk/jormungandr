use super::Error;
use crate::jcli_app::utils::{io, OutputFormat};
use chain_vote::EncryptedTally;
use serde::Serialize;
use std::io::BufRead;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct TallyDecryptWithAllShares {
    /// The path to hex-encoded encrypted tally state. If this parameter is not
    /// specified, the encrypted tally state will be read from the standard
    /// input.
    #[structopt(long = "tally")]
    encrypted_tally: Option<PathBuf>,
    /// The minimum number of shares needed for decryption
    #[structopt(long = "threshold", default_value = "3")]
    threshold: usize,
    /// Maximum supported number of votes
    #[structopt(long = "maxvotes")]
    max_votes: u64,
    /// Computing table cache size, usually total_votes/number_of_options
    #[structopt(long = "table_size")]
    table_size: usize,
    /// The path to encoded necessary shares. If this parameter is not
    /// specified, the shares will be read from the standard input.
    #[structopt(long = "shares")]
    shares: Option<PathBuf>,
    #[structopt(flatten)]
    output_format: OutputFormat,
}

#[derive(Serialize)]
struct Output {
    result: Vec<Option<u64>>,
}

impl TallyDecryptWithAllShares {
    pub fn exec(&self) -> Result<(), Error> {
        let encrypted_tally_hex = io::read_line(&self.encrypted_tally)?;
        let encrypted_tally_bytes = base64::decode(encrypted_tally_hex)?;
        let encrypted_tally =
            EncryptedTally::from_bytes(&encrypted_tally_bytes).ok_or(Error::EncryptedTallyRead)?;

        let mut shares_file = io::open_file_read(&self.shares)?;

        let shares: Vec<chain_vote::TallyDecryptShare> = {
            let mut shares = Vec::with_capacity(self.threshold);
            for _ in 0..self.threshold {
                let mut buff = String::new();
                &shares_file.read_line(&mut buff);
                shares.push(
                    chain_vote::TallyDecryptShare::from_bytes(&base64::decode(buff)?)
                        .ok_or(Error::DecryptionShareRead)?,
                );
            }
            shares
        };

        let state = encrypted_tally.state();
        let result = chain_vote::result(self.max_votes, self.table_size, &state, &shares);
        let output = self
            .output_format
            .format_json(serde_json::to_value(Output {
                result: result.votes,
            })?)?;

        println!("{}", output);

        Ok(())
    }
}
