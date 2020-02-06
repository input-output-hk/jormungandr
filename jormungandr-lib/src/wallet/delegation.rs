use crate::{
    crypto::{
        account::Identifier as AccountIdentifier,
        hash::Hash,
        key::{Identifier, SigningKey},
    },
    interfaces::Address,
};
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    key::EitherEd25519SecretKey,
    transaction::{TransactionSignDataHash, Witness},
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
    delegations: Vec<AccountIdentifier>,
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

    pub fn generate_new_signing_key(&mut self, delegation: AccountIdentifier) -> &SpendingKey {
        let key = SigningKey::generate(&mut self.rng);
        self.signing_keys.push(key);
        self.delegations.push(delegation);
        self.signing_keys.get(self.signing_keys.len() - 1).unwrap()
    }

    pub fn delegation(&self, i: usize) -> &AccountIdentifier {
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

    pub fn identifier(&self) -> Identifier<chain_crypto::Ed25519> {
        self.last_signing_key().identifier()
    }

    pub fn signing_key(&self, i: usize) -> &SpendingKey {
        self.signing_keys.get(i).expect("no signing key found")
    }

    pub fn last_delegation_identifier(&self) -> AccountIdentifier {
        let index = self.delegations.len() - 1;
        self.delegations.get(index).unwrap().clone()
    }

    pub fn last_signing_key(&self) -> &SpendingKey {
        let index = self.signing_keys.len() - 1;
        self.signing_keys.get(index).expect("no signing key found")
    }

    pub fn mk_witness(
        &self,
        block0_hash: &Hash,
        signing_data: &TransactionSignDataHash,
        i: usize,
    ) -> Witness {
        Witness::new_utxo(&block0_hash.clone().into_hash(), signing_data, |d| {
            self.last_signing_key().as_ref().sign(d)
        })
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
