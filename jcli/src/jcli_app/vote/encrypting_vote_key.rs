use chain_vote::EncryptingVoteKey;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct BuildEncryptingVoteKey {
    pub threshold: usize,
    pub input: Vec<String>,
}

impl BuildEncryptingVoteKey {
    pub fn exec(&self) -> Result<(), super::Error> {
        let keys: Vec<chain_vote::MemberCommunicationPublicKey> =
            self.input.iter().map(|k| key_from_string(k)).collect();
        let election_public_key = generate_states(self.threshold, &keys);
        println!("{}", hex::encode(election_public_key.to_bytes()));
        Ok(())
    }
}

fn key_from_string(key: &str) -> chain_vote::MemberCommunicationPublicKey {
    // TODO: remove unwrap and bubble up error
    let raw_key = hex::decode(key).unwrap();
    chain_vote::MemberCommunicationPublicKey::from_public_key(chain_vote::gargamel::PublicKey {
        pk: chain_vote::CRS::from_bytes(&raw_key).unwrap(),
    })
}

fn generate_states(
    threshold: usize,
    members_keys: &[chain_vote::MemberCommunicationPublicKey],
) -> chain_vote::committee::ElectionPublicKey {
    let mut rng = ChaCha20Rng::from_entropy();
    let csr = chain_vote::CRS::random(&mut rng);
    let participants: Vec<chain_vote::committee::MemberPublicKey> = (0..members_keys.len())
        .map(|i| chain_vote::MemberState::new(&mut rng, threshold, &csr, &members_keys, i))
        .map(|state| state.public_key())
        .collect();
    chain_vote::EncryptingVoteKey::from_participants(participants.as_slice())
}
