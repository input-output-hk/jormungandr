use chain_addr::{Address, Discrimination, Kind, KindType};
use chain_impl_mockchain::{
    account::SpendingCounter,
    key::{EitherEd25519SecretKey, SpendingPublicKey},
    transaction::{Input, Output, UtxoPointer},
    value::Value,
};
use std::fmt::{Debug, Formatter, Result};

#[derive(Clone)]
pub struct AddressData {
    pub private_key: EitherEd25519SecretKey,
    pub public_key: SpendingPublicKey,
    pub spending_counter: Option<SpendingCounter>,
    pub address: Address,
}

impl Debug for AddressData {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "[{:?},{:?},{:?}]",
            self.public_key, self.spending_counter, self.address
        )
    }
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

    pub fn as_input(&self, value: Value, utxo: UtxoPointer) -> Input {
        match self.address.kind() {
            Kind::Account { .. } => {
                Input::from_account_public_key(self.public_key.clone(), value.clone())
            }
            _ => Input::from_utxo(utxo),
        }
    }

    pub fn as_output(&self, value: Value) -> Output<Address> {
        Output::from_address(self.address.clone(), value)
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
