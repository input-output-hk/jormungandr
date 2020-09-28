use chain_vote::gargamel::PublicKey;
use chain_vote::{committee::MemberSecretKey, MemberCommunicationPublicKey, MemberState};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct CommitteeMemberKey {
    pub threshold: usize,
    pub my: usize,
    pub input: Vec<String>,
}

impl CommitteeMemberKey {
    pub fn exec(&self) -> Result<(), super::Error> {
        let keys: Vec<MemberCommunicationPublicKey> = self
            .input
            .iter()
            .map(|s| str_to_member_communication_public_key(s))
            .collect();
        let member_state = generate_committee_member_keys(self.threshold, self.my, &keys);
        println!(
            "Private: {}",
            hex::encode(member_state.secret_key().to_bytes())
        );
        println!(
            "Public: {}",
            hex::encode(member_state.public_key().to_bytes())
        );
        Ok(())
    }
}

fn str_to_member_communication_public_key(key: &str) -> MemberCommunicationPublicKey {
    // TODO: remove unwraps and bubble up errors
    let raw_key = hex::decode(key).unwrap();
    let pk = PublicKey::from_bytes(&raw_key).unwrap();
    MemberCommunicationPublicKey::from_public_key(pk)
}

fn generate_committee_member_keys(
    threshold: usize,
    my: usize,
    keys: &[MemberCommunicationPublicKey],
) -> MemberState {
    let mut rng = ChaCha20Rng::from_entropy();
    let crs = chain_vote::CRS::random(&mut rng);
    MemberState::new(&mut rng, threshold, &crs, keys, my)
}

#[cfg(test)]
mod test {
    use chain_vote::gargamel::PublicKey;
    use chain_vote::{MemberCommunicationKey, MemberCommunicationPublicKey};
    use rand_chacha::rand_core::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_keys_dance() {
        let mut rng = ChaCha20Rng::from_seed([0u8; 32]);
        let mck = MemberCommunicationKey::new(&mut rng);
        let pk_bytes = mck.to_public().to_bytes();
        let pk = PublicKey::from_bytes(&pk_bytes).unwrap();
        MemberCommunicationPublicKey::from_public_key(pk);
    }
}
