use crate::{
    jcli_lib::vote::Error,
    rest::{v0::message::post_fragment, RestArgs},
    utils::{io, key_parser::read_secret_key},
};
use chain_core::property::Serialize;
use chain_impl_mockchain::{
    fragment::Fragment,
    key::{BftLeaderId, EitherEd25519SecretKey},
    update::{
        SignedUpdateProposal, UpdateProposal as UpdateProposalLib, UpdateProposalWithProposer,
    },
};
use jormungandr_lib::interfaces::ConfigParams;
use std::{io::BufRead, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct UpdateProposal {
    /// the file path to the config file defining the config param changes
    /// If omitted it will be read from the standard input.
    #[structopt(name = "CONFIG_UPDATE")]
    config_file: Option<PathBuf>,

    /// the file path to the file to read the signing key from.
    /// If omitted it will be read from the standard input.
    #[structopt(long)]
    secret: Option<PathBuf>,

    #[structopt(flatten)]
    rest_args: RestArgs,
}

impl UpdateProposal {
    pub fn exec(self) -> Result<(), Error> {
        let secret_key = read_secret_key(self.secret)?;

        let reader = open_config_file(self.config_file)?;

        let config: ConfigParams =
            serde_yaml::from_reader(reader).map_err(Error::ConfigFileCorrupted)?;

        let fragment = build_fragment(secret_key, config);

        let fragment_id = post_fragment(self.rest_args, fragment)?;
        println!("Posted fragment id: {}", fragment_id);

        Ok(())
    }
}

fn open_config_file(config_file: Option<PathBuf>) -> Result<impl BufRead, Error> {
    io::open_file_read(&config_file).map_err(|source| Error::InputInvalid {
        source,
        path: config_file.unwrap_or_default(),
    })
}

fn build_fragment(secret_key: EitherEd25519SecretKey, config: ConfigParams) -> Fragment {
    let proposer_id = secret_key.to_public();

    let update_proposal = UpdateProposalLib::new(config.into());

    let bytes = update_proposal.serialize_as_vec().unwrap();

    let signed_update_proposal = SignedUpdateProposal::new(
        secret_key.sign_slice(bytes.as_slice()),
        UpdateProposalWithProposer::new(update_proposal, BftLeaderId::from(proposer_id)),
    );

    Fragment::UpdateProposal(signed_update_proposal)
}
