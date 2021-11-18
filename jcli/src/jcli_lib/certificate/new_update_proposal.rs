use crate::jcli_lib::{
    certificate::{write_cert, Error},
    utils::{io, key_parser::parse_pub_key},
};
use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::certificate::{self, Certificate};
use jormungandr_lib::interfaces::ConfigParams;
use std::{io::BufRead, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct UpdateProposal {
    /// the proposer ID.
    #[structopt(name = "PROPOSER_ID", parse(try_from_str = parse_pub_key))]
    proposer_id: PublicKey<Ed25519>,

    /// the file path to the config file defining the config param changes
    /// If omitted it will be read from the standard input.
    #[structopt(name = "CONFIG_FILE")]
    config_file: Option<PathBuf>,

    /// print the output signed certificate in the given file, if no file given
    /// the output will be printed in the standard output
    output: Option<PathBuf>,
}

impl UpdateProposal {
    pub fn exec(self) -> Result<(), Error> {
        let reader = open_config_file(self.config_file)?;

        let configs: ConfigParams =
            serde_yaml::from_reader(reader).map_err(Error::ConfigFileCorrupted)?;

        let update_proposal =
            certificate::UpdateProposal::new(configs.into(), self.proposer_id.into());
        let cert = Certificate::UpdateProposal(update_proposal);
        write_cert(self.output.as_deref(), cert.into())
    }
}

fn open_config_file(config_file: Option<PathBuf>) -> Result<impl BufRead, Error> {
    io::open_file_read(&config_file).map_err(|source| Error::InputInvalid {
        source,
        path: config_file.unwrap_or_default(),
    })
}
