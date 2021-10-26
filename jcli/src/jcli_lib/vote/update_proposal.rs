use crate::{
    jcli_lib::vote::Error,
    rest::{v0::message::post_fragment, RestArgs},
    utils::key_parser::read_secret_key,
};
use chain_core::{
    mempack::ReadError,
    property::{Deserialize, Serialize},
};
use chain_impl_mockchain::{
    fragment::{ConfigParams, Fragment},
    key::{BftLeaderId, EitherEd25519SecretKey},
    update::{
        SignedUpdateProposal, UpdateProposal as UpdateProposalLib, UpdateProposalWithProposer,
    },
};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct UpdateProposal {
    /// the config update in the hex representation 
    #[structopt(name = "CONFIG_UPDATE", parse(try_from_str = parse_hex))]
    config: ConfigParams,

    /// the file path to the file to read the signing key from.
    /// If omitted it will be read from the standard input.
    #[structopt(long)]
    secret: Option<PathBuf>,

    #[structopt(flatten)]
    rest_args: RestArgs,
}

fn parse_hex(str: &str) -> Result<ConfigParams, ReadError> {
    let bytes = hex::decode(str).map_err(|err| ReadError::InvalidData(err.to_string()))?;

    ConfigParams::deserialize(bytes.as_slice())
        .map_err(|err| ReadError::InvalidData(err.to_string()))
}

impl UpdateProposal {
    pub fn exec(self) -> Result<(), Error> {
        let secret_key = read_secret_key(self.secret)?;

        let fragment = build_fragment(secret_key, self.config);

        let fragment_id = post_fragment(self.rest_args, fragment)?;
        println!("Posted fragment id: {}", fragment_id);

        Ok(())
    }
}

fn build_fragment(secret_key: EitherEd25519SecretKey, config: ConfigParams) -> Fragment {
    let proposer_id = secret_key.to_public();

    let update_proposal = UpdateProposalLib::new(config);

    let bytes = update_proposal.serialize_as_vec().unwrap();

    let signed_update_proposal = SignedUpdateProposal::new(
        secret_key.sign_slice(bytes.as_slice()),
        UpdateProposalWithProposer::new(update_proposal, BftLeaderId::from(proposer_id)),
    );

    Fragment::UpdateProposal(signed_update_proposal)
}
