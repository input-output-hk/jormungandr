use chain_vote::MemberCommunicationKey;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct CommitteeCommunicationKey {}

impl CommitteeCommunicationKey {
    pub fn exec(&self) -> Result<(), super::Error> {
        let key = generate_committee_member_keys();
        println!("Private: {}", hex::encode(key.to_bytes()));
        println!("Public: {}", hex::encode(key.to_public().to_bytes()));
        Ok(())
    }
}

fn generate_committee_member_keys() -> MemberCommunicationKey {
    let mut rng = ChaCha20Rng::from_entropy();
    MemberCommunicationKey::new(&mut rng)
}
