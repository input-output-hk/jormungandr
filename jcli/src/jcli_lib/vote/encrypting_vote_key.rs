use crate::jcli_lib::vote::{Error, OutputFile};
use bech32::{FromBase32, ToBase32};
use std::io::Write as _;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct EncryptingVoteKey {
    /// Keys of all committee members
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
        writeln!(
            output,
            "{}",
            bech32::encode(
                crate::jcli_lib::vote::bech32_constants::ENCRYPTING_VOTE_PK_HRP,
                election_public_key.to_bytes().to_base32()
            )
            .map_err(Error::Bech32)?
        )
        .map_err(Error::from)
    }
}

fn parse_member_key(key: &str) -> Result<chain_vote::committee::MemberPublicKey, Error> {
    bech32::decode(key)
        .map_err(Error::from)
        .and_then(|(hrp, raw_key)| {
            if hrp != crate::jcli_lib::vote::bech32_constants::MEMBER_PK_HRP {
                return Err(Error::InvalidPublicKey);
            }
            chain_vote::encryption::PublicKey::from_bytes(
                &Vec::<u8>::from_base32(&raw_key).map_err(|_| Error::InvalidPublicKey)?,
            )
            .ok_or(Error::InvalidPublicKey)
        })
        .map(chain_vote::committee::MemberPublicKey::from)
}
