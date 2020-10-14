use super::Error;
use chain_vote::EncryptingVoteKey;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct MakeEncryptingVoteKey {
    #[structopt(
        parse(try_from_str = parse_member_key)
    )]
    member_keys: Vec<chain_vote::committee::MemberPublicKey>,
}

fn parse_member_key(key: &str) -> Result<chain_vote::committee::MemberPublicKey, Error> {
    hex::decode(key)
        .map_err(Error::from)
        .and_then(|raw_key| {
            chain_vote::gargamel::PublicKey::from_bytes(&raw_key).ok_or(Error::InvalidPublicKey)
        })
        .map(chain_vote::committee::MemberPublicKey::from_public_key)
}

impl MakeEncryptingVoteKey {
    pub fn exec(&self) -> Result<(), Error> {
        if self.member_keys.is_empty() {
            Err(Error::EncryptingVoteKeyFromEmpty)
        } else {
            let election_public_key = EncryptingVoteKey::from_participants(&self.member_keys);

            println!("{}", hex::encode(election_public_key.to_bytes()));
            Ok(())
        }
    }
}
