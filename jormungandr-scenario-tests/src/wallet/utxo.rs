use jormungandr_lib::{crypto::key, interfaces::UTxOInfo};
use rand_chacha::ChaChaRng;
use rand_core::{CryptoRng, RngCore, SeedableRng};

pub type SpendingKey = key::SigningKey<chain_crypto::Ed25519>;

/// wallet for an account
#[derive(Debug, Clone)]
pub struct Wallet {
    /// this is the root seed of the wallet, everytime we will require
    /// the wallet to update we will update the rng, we keep the `seed`
    /// so we may reproduce the steps of the wallet
    seed: [u8; 32],

    rng: ChaChaRng,

    /// the spending key
    signing_keys: Vec<SpendingKey>,

    /// utxos with the index in the `signing_keys` so we can later
    /// sign the witness for the next transaction,
    utxos: Vec<(usize, UTxOInfo)>,
}

impl Wallet {
    pub fn generate<RNG>(rng: &mut RNG) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        let mut seed = [0; 32];
        rng.fill_bytes(&mut seed);

        seed.into()
    }

    pub fn generate_new_signing_key(&mut self) -> &SpendingKey {
        let key = key::SigningKey::generate(&mut self.rng);

        self.signing_keys.push(key);

        self.signing_keys.get(self.signing_keys.len() - 1).unwrap()
    }
}

impl From<[u8; 32]> for Wallet {
    fn from(seed: [u8; 32]) -> Self {
        let mut wallet = Wallet {
            signing_keys: Vec::new(),
            seed: seed.clone(),
            rng: ChaChaRng::from_seed(seed),
            utxos: Vec::new(),
        };

        wallet.generate_new_signing_key();

        wallet
    }
}
