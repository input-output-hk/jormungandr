use super::Error;
use crate::jcli_app::utils::{io, OutputFormat};
use std::path::PathBuf;
use structopt::StructOpt;

pub struct TallyDecryptWithAllShares {
    /// The path to hex-encoded encrypted tally state. If this parameter is not
    /// specified, the encrypted tally state will be read from the standard
    /// input.
    #[structopt(long = "tally")]
    encrypted_tally: Option<PathBuf>,
    #[structopt(long = "threshold", default = 3)]
    threshold: usize,
    #[strcturopt(long = "maxvotes")]
    max_votes: u64,
    #[structopt(long = "table_size")]
    table_size: usize,
    /// The path to encoded necessare shares. If this parameter is not
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
        let encrypted_tally_bytes = hex::decode(encrypted_tally_hex)?;
        let encrypted_tally =
            Tally::from_bytes(&encrypted_tally_bytes).ok_or(Error::EncryptedTallyRead)?;
        let mut shares_file = io::open_file_read(&self.shares)?;
        let shares: Vec<chain_vote::TallyDecryptShare> = (0..self.threshold)
            .map(|_| {
                let mut buff = String::new();
                &shares_file.read_line(&mut buff);
                chain_vote::TallyDecryptShare::from_bytes(&hex::decode(buff)?)
            })
            .collect();
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
