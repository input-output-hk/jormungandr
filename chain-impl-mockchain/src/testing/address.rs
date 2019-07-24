use crate::{
    account::SpendingCounter,
    key::{EitherEd25519SecretKey},
    transaction::{Input, Output},
    utxo::Entry,
    value::Value,
};
use chain_addr::{Address, Discrimination, Kind, KindType,AddressReadable};
use chain_crypto::{Ed25519,Ed25519Extended, PublicKey,bech32::Bech32,testing::TestCryptoGen,AsymmetricKey,KeyPair};
use std::fmt::{self, Debug};
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicU64, Ordering};

///
/// Struct is responsible for adding some code which makes converting into transaction input/output easily.
/// Also it held all needed information (private key, public key) which can construct witness for transaction.
///
#[derive(Clone)]
pub struct AddressData {
    private_key: EitherEd25519SecretKey,
    pub spending_counter: Option<SpendingCounter>,
    pub address: Address,
}

impl Debug for AddressData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AddressData")
            .field("public_key", &self.public_key())
            .field("spending_counter", &self.spending_counter)
            .field("address", &self.address)
            .finish()
    }
}

impl PartialEq for AddressData {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

lazy_static! {
    static ref DETERMINISTIC_ID: AtomicU64 = AtomicU64::new(1);
}

impl AddressData {
    pub fn new(
        private_key: EitherEd25519SecretKey,
        spending_counter: Option<SpendingCounter>,
        address: Address,
    ) -> Self {
        AddressData {
            private_key,
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
        let (sk,pk) = AddressData::generate_key_pair::<Ed25519Extended>().into_keys();
        let sk = EitherEd25519SecretKey::Extended(sk);
        let user_address = Address(discrimination.clone(), Kind::Single(pk.clone()));
        AddressData::new(sk, None, user_address)
    }

    pub fn account(discrimination: Discrimination) -> Self {
        let (sk,pk) = AddressData::generate_key_pair::<Ed25519Extended>().into_keys();
        let sk = EitherEd25519SecretKey::Extended(sk);
        let user_address = Address(discrimination.clone(), Kind::Account(pk.clone()));
        AddressData::new(sk, Some(SpendingCounter::zero()), user_address)
    }

    pub fn delegation(discrimination: Discrimination) -> Self {
        let (single_sk,single_pk) = AddressData::generate_key_pair::<Ed25519Extended>().into_keys();
        let (_delegation_sk,delegation_pk) = AddressData::generate_key_pair::<Ed25519Extended>().into_keys();

        let user_address = Address(
            discrimination.clone(),
            Kind::Group(single_pk.clone(), delegation_pk.clone()),
        );
        let single_sk = EitherEd25519SecretKey::Extended(single_sk);
        AddressData::new(single_sk, None, user_address)
    }

    pub fn make_input(&self, value: Value, utxo: Option<Entry<Address>>) -> Input {
        match self.address.kind() {
            Kind::Account { .. } => {
                Input::from_account_public_key(self.public_key(), value.clone())
            }
            Kind::Single { .. } | Kind::Group { .. } | Kind::Multisig { .. } => {
                Input::from_utxo_entry(utxo.expect(&format!(
                    "invalid state, utxo should be Some if Kind not Account {:?}",
                    &self.address
                )))
            }
        }
    }

    pub fn make_output(&self, value: Value) -> Output<Address> {
        Output::from_address(self.address.clone(), value)
    }

    pub fn public_key(&self) -> PublicKey<Ed25519> {
        match self.kind() {
            Kind::Account(key) => key,
            Kind::Group(key, _) => key,
            Kind::Single(key) => key,
            Kind::Multisig(_) => panic!("not yet implemented"),
        }
    }

    pub fn private_key(&self) -> EitherEd25519SecretKey{
        self.private_key.clone()
    }

    pub fn kind(&self) -> Kind {
        self.address.kind().clone()
    }

    pub fn discrimination(&self) -> Discrimination {
        self.address.discrimination().clone()
    }

    pub fn address_as_string(&self) -> String {
        let prefix = match self.discrimination() {
            Discrimination::Production => "ta",
            Discrimination::Test => "ca"
        };
        AddressReadable::from_address(prefix, &self.address).to_string()
    }

    pub fn public_key_as_string(&self) -> String {
        self.public_key().to_bech32_str()
    }

    pub fn generate_key_pair<A: AsymmetricKey>() -> KeyPair<A>{
        let n = DETERMINISTIC_ID.fetch_add(1, Ordering::SeqCst) as u32;
        TestCryptoGen(0).keypair::<A>(n)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AddressDataValue {
    pub address_data: AddressData,
    pub value: Value,
}

impl AddressDataValue {
    pub fn new(address_data: AddressData, value: Value) -> Self {
        AddressDataValue {
            address_data: address_data,
            value: value,
        }
    }

    pub fn make_input(&self, utxo: Option<Entry<Address>>) -> Input {
        self.address_data.make_input(self.value, utxo)
    }

    pub fn make_output(&self) -> Output<Address> {
        self.address_data.make_output(self.value)
    }
}
