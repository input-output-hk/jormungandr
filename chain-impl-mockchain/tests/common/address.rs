use chain_addr::{Address, Discrimination, Kind, KindType};
use chain_impl_mockchain::account::SpendingCounter;
use chain_impl_mockchain::key::{EitherEd25519SecretKey, SpendingPublicKey};

pub struct AddressData {
    pub private_key: EitherEd25519SecretKey,
    pub public_key: SpendingPublicKey,
    pub spending_counter: Option<SpendingCounter>,
    pub address: Address,
}

impl AddressData {
    pub fn new(
        private_key: EitherEd25519SecretKey,
        public_key: SpendingPublicKey,
        spending_counter: Option<SpendingCounter>,
        address: Address,
    ) -> Self {
        AddressData {
            private_key,
            public_key,
            address,
            spending_counter,
        }
    }

    pub fn from_discrimination_and_kind_type(
        discrimination: Discrimination,
        kind: &KindType,
    ) -> Self {
        match kind {
            KindType::Account => AddressData::account(discrimination),
            KindType::Single => AddressData::utxo(discrimination),
            KindType::Group => AddressData::delegation(discrimination),
            _ => panic!("not implemented yet"),
        }
    }

    pub fn utxo(discrimination: Discrimination) -> Self {
        let sk = AddressData::generate_random_secret_key();
        let pk = sk.to_public();
        let user_address = Address(discrimination.clone(), Kind::Single(pk.clone()));
        AddressData::new(sk, pk, None, user_address)
    }

    pub fn account(discrimination: Discrimination) -> Self {
        let sk = AddressData::generate_random_secret_key();
        let pk = sk.to_public();
        let user_address = Address(discrimination.clone(), Kind::Account(pk.clone()));
        AddressData::new(sk, pk, Some(SpendingCounter::zero()), user_address)
    }

    pub fn delegation(discrimination: Discrimination) -> Self {
        let single_sk = AddressData::generate_random_secret_key();
        let single_pk = single_sk.to_public();

        let delegation_sk = AddressData::generate_random_secret_key();
        let delegation_pk = delegation_sk.to_public();

        let user_address = Address(
            discrimination.clone(),
            Kind::Group(single_pk.clone(), delegation_pk.clone()),
        );
        AddressData::new(single_sk, single_pk, None, user_address)
    }

    fn generate_random_secret_key() -> EitherEd25519SecretKey {
        EitherEd25519SecretKey::generate(rand_os::OsRng::new().unwrap())
    }
}
