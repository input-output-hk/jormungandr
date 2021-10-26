use crate::{
    jcli_lib::vote::Error,
    rest::{v0::message::post_fragment, RestArgs},
    utils::key_parser::read_secret_key,
};
use chain_core::property::Serialize;
use chain_impl_mockchain::{
    fragment::Fragment,
    key::{BftLeaderId, EitherEd25519SecretKey},
    update::{SignedUpdateVote, UpdateProposalId, UpdateVote as UpdateVoteLib},
};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct UpdateVote {
    /// the Proposal ID of the proposal.
    #[structopt(name = "PROPOSAL_ID")]
    proposal_id: UpdateProposalId,

    /// the file path to the file to read the signing key from.
    /// If omitted it will be read from the standard input.
    #[structopt(long)]
    secret: Option<PathBuf>,

    #[structopt(flatten)]
    rest_args: RestArgs,
}

impl UpdateVote {
    pub fn exec(self) -> Result<(), Error> {
        let secret_key = read_secret_key(self.secret)?;

        let fragment = build_fragment(secret_key, self.proposal_id);

        let fragment_id = post_fragment(self.rest_args, fragment)?;
        println!("Posted fragment id: {}", fragment_id);

        Ok(())
    }
}

fn build_fragment(secret_key: EitherEd25519SecretKey, proposal_id: UpdateProposalId) -> Fragment {
    let voter_id = secret_key.to_public();

    let update_vote = UpdateVoteLib::new(proposal_id, BftLeaderId::from(voter_id));

    let bytes = update_vote.serialize_as_vec().unwrap();

    let signed_update_vote =
        SignedUpdateVote::new(secret_key.sign_slice(bytes.as_slice()), update_vote);

    Fragment::UpdateVote(signed_update_vote)
}
