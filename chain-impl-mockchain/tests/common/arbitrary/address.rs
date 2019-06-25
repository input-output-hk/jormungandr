use crate::common::address::AddressData;
use chain_addr::{Discrimination, KindType};
use quickcheck::{Arbitrary, Gen};

#[derive(Clone, Debug)]
pub struct ArbitraryAddressKind(pub KindType);

impl Arbitrary for ArbitraryAddressKind {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        match u8::arbitrary(g) % 3 {
            0 => ArbitraryAddressKind(KindType::Single),
            1 => ArbitraryAddressKind(KindType::Group),
            2 => ArbitraryAddressKind(KindType::Account),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArbitraryAddressesData(pub Vec<AddressData>);

impl Arbitrary for ArbitraryAddressesData {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let count = u8::arbitrary(gen);
        let mut addresses = vec![];
        for _ in 0..count {
            addresses.push(AddressData::arbitrary(gen));
        }
        ArbitraryAddressesData(addresses)
    }
}

impl Arbitrary for AddressData {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let kind = ArbitraryAddressKind::arbitrary(gen);
        AddressData::from_discrimination_and_kind_type(Discrimination::Test, &kind.0)
    }
}
