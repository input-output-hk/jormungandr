use crate::{
    testing::{
        arbitrary::kind_type::KindTypeWithoutMultisig,
        arbitrary::AverageValue,
        data::{AddressData, AddressDataValue},
        ledger::TestLedger
    },
    transaction::{Input, Output},
    value::Value,
};
use chain_addr::{Address, Discrimination, Kind};
use quickcheck::{Arbitrary, Gen};
use std::iter;

#[derive(Clone, Debug)]
pub struct ArbitraryAddressDataVec(pub Vec<AddressData>);

impl Arbitrary for ArbitraryAddressDataVec {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let size_limit = 253;
        let n = usize::arbitrary(gen) % size_limit + 1;
        let addresses = iter::from_fn(|| Some(AddressData::arbitrary(gen))).take(n);
        ArbitraryAddressDataVec(addresses.collect())
    }
}

#[derive(Clone, Debug)]
pub struct ArbitraryAddressDataValueVec(pub Vec<AddressDataValue>);

impl Arbitrary for ArbitraryAddressDataValueVec {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let size_limit = 10;
        let n = usize::arbitrary(gen) % size_limit + 1;
        let addresses = iter::from_fn(|| Some(AddressDataValue::arbitrary(gen))).take(n);
        ArbitraryAddressDataValueVec(addresses.collect())
    }
}

impl ArbitraryAddressDataValueVec {
    pub fn iter(&self) -> std::slice::Iter<AddressDataValue> {
        self.0.iter()
    }

    pub fn values(&self) -> Vec<AddressDataValue> {
        self.0.clone()
    }

    pub fn as_addresses(&self) -> Vec<AddressData> {
        self.iter().cloned().map(|x| x.address_data).collect()
    }

    pub fn make_outputs(&self) -> Vec<Output<Address>> {
        self.iter().map(|x| x.make_output()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.values().len() == 0
    }

    pub fn total_value(&self) -> Value {
        Value::sum(self.iter().map(|input| input.value)).unwrap()
    }

    pub fn make_inputs(&self, ledger: &TestLedger) -> Vec<Input> {
        self.iter()
            .map(|x| {
                let utxo = ledger.find_utxo_for_address(&x.clone().into());
                x.make_input(utxo)
            })
            .collect()
    }
}

impl Arbitrary for AddressData {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let kind_without_multisig = KindTypeWithoutMultisig::arbitrary(gen);
        AddressData::from_discrimination_and_kind_type(
            Discrimination::Test,
            &kind_without_multisig.kind_type(),
        )
    }
}

impl Arbitrary for AddressDataValue {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        AddressDataValue::new(
            AddressData::arbitrary(gen),
            AverageValue::arbitrary(gen).into(),
        )
    }
}

impl ArbitraryAddressDataValueVec {
    pub fn utxos(&self) -> Vec<AddressDataValue> {
        self.0
            .iter()
            .cloned()
            .filter(|x| match x.address_data.kind() {
                Kind::Single { .. } => true,
                _ => false,
            })
            .collect()
    }
    pub fn accounts(&self) -> Vec<AddressDataValue> {
        self.0
            .iter()
            .cloned()
            .filter(|x| match x.address_data.kind() {
                Kind::Account { .. } => true,
                _ => false,
            })
            .collect()
    }

    pub fn delegations(&self) -> Vec<AddressDataValue> {
        self.0
            .iter()
            .cloned()
            .filter(|x| match x.address_data.kind() {
                Kind::Group { .. } => true,
                _ => false,
            })
            .collect()
    }
}
