use chain_crypto::AsymmetricKey;
use jormungandr_lib::crypto::key::KeyPair;

pub fn create_new_key_pair<K: AsymmetricKey>() -> KeyPair<K> {
    KeyPair::generate(rand::rngs::OsRng)
}
