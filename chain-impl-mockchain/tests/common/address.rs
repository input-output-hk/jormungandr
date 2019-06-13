use chain_addr::{Address, Discrimination, Kind};
use chain_impl_mockchain::key::{SpendingPublicKey, SpendingSecretKey};

pub struct AddressData {
    pub private_key: SpendingSecretKey,
    pub public_key: SpendingPublicKey,
    pub address: Address,
}

impl AddressData {
    pub fn new(
        private_key: SpendingSecretKey,
        public_key: SpendingPublicKey,
        address: Address,
    ) -> Self {
        AddressData {
            private_key,
            public_key,
            address,
        }
    }

    pub fn utxo(discrimination: Discrimination) -> AddressData {
        let sk = AddressData::generate_random_secret_key();
        let pk = sk.to_public();
        let user_address = Address(discrimination.clone(), Kind::Single(pk.clone()));
        AddressData::new(sk, pk, user_address)
    }

    pub fn account(discrimination: Discrimination) -> AddressData {
        let sk = AddressData::generate_random_secret_key();
        let pk = sk.to_public();
        let user_address = Address(discrimination.clone(), Kind::Account(pk.clone()));
        AddressData::new(sk, pk, user_address)
    }

    pub fn delegation(discrimination: Discrimination) -> AddressData {
        let single_sk = AddressData::generate_random_secret_key();
        let single_pk = single_sk.to_public();

        let delegation_sk = AddressData::generate_random_secret_key();
        let delegation_pk = delegation_sk.to_public();

        let user_address = Address(
            discrimination.clone(),
            Kind::Group(single_pk.clone(), delegation_pk.clone()),
        );
        AddressData::new(single_sk, single_pk, user_address)
    }

    fn generate_random_secret_key() -> SpendingSecretKey {
        SpendingSecretKey::generate(rand::thread_rng())
    }
}
