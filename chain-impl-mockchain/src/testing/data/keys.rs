use std::collections::HashMap;
use crate::key::EitherEd25519SecretKey;
use chain_crypto::{SecretKey, PublicKey, Ed25519};
use chain_crypto::testing::TestCryptoGen;
use chain_addr::{Address, Discrimination, Kind};

pub struct KeysDb {
    rng: u32,
    tcg: TestCryptoGen,
    ed25519: HashMap<PublicKey<Ed25519>, EitherEd25519SecretKey>,
}

impl KeysDb {
    /// Create a new keys DB
    pub fn new(tcg: TestCryptoGen) -> Self {
        KeysDb {
            rng: 0,
            tcg: tcg,
            ed25519: HashMap::new(),
        }
    }

    pub fn empty() -> Self {
       KeysDb::new(TestCryptoGen(0))
    }

    pub fn add_key(&mut self, sk: EitherEd25519SecretKey) {
        let pk = sk.to_public();
        self.ed25519.insert(pk,sk);
    }

    /// Create a new Ed25519 and record it
    pub fn new_ed25519_secret_key(&mut self) -> SecretKey<Ed25519> {
        let sk = self.tcg.secret_key(self.rng);
        self.rng += 1;

        let pk = sk.to_public();
        self.ed25519.insert(pk, EitherEd25519SecretKey::Normal(sk.clone()));
        sk
    }

    /// same as new_ed25519_secret_key but instead return the public key directly
    pub fn new_ed25519_public_key(&mut self) -> PublicKey<Ed25519> {
        self.new_ed25519_secret_key().to_public()
    }

    pub fn new_account_address(&mut self) -> Address {
        let pk = self.new_ed25519_public_key();
        Address(Discrimination::Test, Kind::Account(pk))
    }

    /// Try to get the associated secret key for a given public key
    pub fn find_ed25519_secret_key<'a>(&'a self, pk: &PublicKey<Ed25519>) -> Option<&'a EitherEd25519SecretKey> {
        self.ed25519.get(pk)
    }

    pub fn find_by_address<'a>(&'a self, addr: &Address) -> Option<&'a EitherEd25519SecretKey> {
        match addr.kind() {
            Kind::Single(pk) => self.find_ed25519_secret_key(pk),
            Kind::Group(pk,_) => self.find_ed25519_secret_key(pk),
            Kind::Account(pk) => self.find_ed25519_secret_key(pk),
            Kind::Multisig(_) => unimplemented!(),
        }
    }
}
