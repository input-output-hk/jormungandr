use crate::testing::{
    arbitrary::kind_type::KindTypeWithoutMultisig,
    arbitrary::AverageValue,
    data::{AddressData, AddressDataValue},
};
use chain_addr::{Discrimination, Kind};
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
            Arbitrary::arbitrary(gen),
            AverageValue::arbitrary(gen).into(),
        )
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
