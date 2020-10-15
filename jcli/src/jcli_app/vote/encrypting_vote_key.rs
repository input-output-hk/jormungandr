use crate::jcli_app::vote::{Error, OutputFile};
use std::io::Write as _;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct EncryptingVoteKey {
    #[structopt(
        parse(try_from_str = parse_member_key),
        required = true,
        short = "k",
        long = "keys"
    )]
    member_keys: Vec<chain_vote::committee::MemberPublicKey>,

    #[structopt(flatten)]
    output_file: OutputFile,
}

impl EncryptingVoteKey {
    pub fn exec(&self) -> Result<(), Error> {
        let election_public_key =
            chain_vote::EncryptingVoteKey::from_participants(&self.member_keys);

        let mut output = self.output_file.open()?;
        writeln!(output, "{}", hex::encode(election_public_key.to_bytes())).map_err(Error::from)
    }
}

fn parse_member_key(key: &str) -> Result<chain_vote::committee::MemberPublicKey, Error> {
    hex::decode(key)
        .map_err(Error::from)
        .and_then(|raw_key| {
            chain_vote::gargamel::PublicKey::from_bytes(&raw_key).ok_or(Error::InvalidPublicKey)
        })
        .map(chain_vote::committee::MemberPublicKey::from)
}
