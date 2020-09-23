use chain_vote::gang::GroupElement;
use chain_vote::gargamel::PublicKey;
use chain_vote::{committee::MemberSecretKey, MemberCommunicationPublicKey, MemberState};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct CommitteeMemberKey {
    pub input: Vec<String>,
    pub threshold: usize,
    pub my: usize,
}

impl CommitteeMemberKey {
    pub fn exec(&self) -> Result<(), super::Error> {
        let keys: Vec<MemberCommunicationPublicKey> = self
            .input
            .iter()
            .map(|s| str_to_member_communication_public_key(s))
            .collect();
        let secret_key = generate_committee_member_keys(self.threshold, self.my, &keys);
        println!("{}", hex::encode(secret_key.to_bytes()));
        Ok(())
    }
}

fn str_to_member_communication_public_key(key: &str) -> MemberCommunicationPublicKey {
    // TODO: remove unwraps and bubble up errors
    let raw_key = hex::decode(key).unwrap();
    MemberCommunicationPublicKey::from_public_key(PublicKey::from_bytes(&raw_key).unwrap())
}

fn generate_committee_member_keys(
    threshold: usize,
    my: usize,
    keys: &[MemberCommunicationPublicKey],
) -> MemberSecretKey {
    let mut rng = ChaCha20Rng::from_entropy();
    let crs = chain_vote::gang::GroupElement::random(&mut rng);
    MemberState::new(&mut rng, threshold, &crs, keys, my)
        .secret_key()
        .clone()
}
