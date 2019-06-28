use chain_addr::{Address, Discrimination, Kind, KindType};
use crate::{
    account::SpendingCounter,
    key::{EitherEd25519SecretKey, SpendingPublicKey},
    transaction::{Input, Output, UtxoPointer},
    value::Value,
};
use std::fmt::{self, Debug};

///
/// Struct is responsible for adding some code which makes converting into transaction input/output easily.
/// Also it held all needed information (private key, public key) which can construct witness for transaction.
///
#[derive(Clone)]
pub struct AddressData {
    pub private_key: EitherEd25519SecretKey,
    pub public_key: SpendingPublicKey,
    pub spending_counter: Option<SpendingCounter>,
    pub address: Address,
}

impl Debug for AddressData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AddressData")
            .field("public_key", &self.public_key)
            .field("spending_counter", &self.spending_counter)
            .field("address", &self.address)
            .finish()
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

    pub fn make_input(&self, value: Value, utxo: UtxoPointer) -> Input {
        match self.address.kind() {
            Kind::Account { .. } => {
                Input::from_account_public_key(self.public_key.clone(), value.clone())
            }
            Kind::Single { .. } | Kind::Group { .. } | Kind::Multisig { .. } => {
                Input::from_utxo(utxo)
            }
        }
    }

    pub fn make_output(&self, value: Value) -> Output<Address> {
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
