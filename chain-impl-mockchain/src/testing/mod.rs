pub mod arbitrary;
pub mod address;
pub mod ledger;
pub mod tx_builder;

pub use arbitrary::*;

use chain_crypto::{AsymmetricKey,KeyPair};

pub fn generate_key_pair<A: AsymmetricKey>() -> KeyPair<A>{
    let mut rng = rand_os::OsRng::new().unwrap();
    KeyPair::generate(&mut rng)
}
