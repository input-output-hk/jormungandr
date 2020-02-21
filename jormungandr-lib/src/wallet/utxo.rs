use crate::{
    crypto::{
        hash::Hash,
        key::{self, Identifier},
    },
    interfaces::{Address, UTxOInfo},
};
use chain_addr::Discrimination;
use chain_impl_mockchain::transaction::{TransactionSignDataHash, Witness};
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
        self.last_signing_key()
    }

    pub fn address(&self, discrimination: Discrimination) -> Address {
        self.address_nth(0, discrimination)
    }

    pub fn address_nth(&self, i: usize, discrimination: Discrimination) -> Address {
        self.signing_key(i)
            .identifier()
            .to_single_address(discrimination)
            .into()
    }

    pub fn identifier(&self) -> Identifier<chain_crypto::Ed25519> {
        self.last_signing_key().identifier()
    }

    pub fn signing_key(&self, i: usize) -> &SpendingKey {
        self.signing_keys.get(i).expect("no signing key found")
    }

    pub fn last_signing_key(&self) -> &SpendingKey {
        let index = self.signing_keys.len() - 1;
        self.signing_keys.get(index).expect("no signing key found")
    }

    pub fn mk_witness(
        &self,
        block0_hash: &Hash,
        signing_data: &TransactionSignDataHash,
        _i: usize,
    ) -> Witness {
        Witness::new_utxo(&block0_hash.clone().into_hash(), signing_data, |d| {
            self.last_signing_key().as_ref().sign(d)
        })
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
