use crate::{
    crypto::{account::Identifier, hash::Hash, key::SigningKey},
    interfaces::Address,
};
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    key::EitherEd25519SecretKey,
    transaction::{TransactionSignDataHash, UnspecifiedAccountIdentifier, Witness},
};
use rand_chacha::ChaChaRng;
use rand_core::{CryptoRng, RngCore, SeedableRng};
pub type SpendingKey = SigningKey<chain_crypto::Ed25519>;

/// wallet for an delegation
#[derive(Debug, Clone)]
pub struct Wallet {
    /// this is the root seed of the wallet, everytime we will require
    /// the wallet to update we will update the rng, we keep the `seed`
    /// so we may reproduce the steps of the wallet
    seed: [u8; 32],

    rng: ChaChaRng,

    /// the spending key
    signing_keys: Vec<SpendingKey>,

    /// the identifier of delegated account
    delegations: Vec<Identifier>,
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

    pub fn generate_new_signing_key(&mut self, delegation: Identifier) -> &SpendingKey {
        let key = SigningKey::generate(&mut self.rng);
        self.signing_keys.push(key);
        self.delegations.push(delegation);
        self.signing_keys.get(self.signing_keys.len() - 1).unwrap()
    }

    pub fn delegation(&self, i: usize) -> &Identifier {
        &self.delegations.get(i).unwrap()
    }

    pub fn address(&self, discrimination: Discrimination) -> Address {
        self.address_nth(0, discrimination)
    }

    pub fn address_nth(&self, i: usize, discrimination: Discrimination) -> Address {
        self.signing_key(i)
            .identifier()
            .to_group_address(discrimination, self.delegation(i).clone().to_inner().into())
            .into()
    }

    pub fn signing_key(&self, i: usize) -> &SpendingKey {
        self.signing_keys.get(i).expect("no signing key found")
    }

    pub fn mk_witness(
        &self,
        block0_hash: &Hash,
        signing_data: &TransactionSignDataHash,
        i: usize,
    ) -> Witness {
        let secret_key =
            EitherEd25519SecretKey::Normal(self.signing_key(i).clone().into_secret_key());

        Witness::new_utxo(&block0_hash.clone().into_hash(), signing_data, &secret_key)
    }
}

impl From<[u8; 32]> for Wallet {
    fn from(seed: [u8; 32]) -> Self {
        Wallet {
            signing_keys: Vec::new(),
            seed: seed.clone(),
            rng: ChaChaRng::from_seed(seed),
            delegations: Vec::new(),
        }
    }
}
