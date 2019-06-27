use crate::common::{address::AddressData, arbitrary::kind_type::KindTypeWithoutMultisig};
use chain_addr::{Discrimination, KindType};
use quickcheck::{Arbitrary, Gen};

#[derive(Clone, Debug)]
pub struct ArbitraryAddressDataCollection(pub Vec<AddressData>);

impl Arbitrary for ArbitraryAddressDataCollection {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let count = u8::arbitrary(gen);
        let mut addresses = vec![];
        for _ in 0..count {
            addresses.push(AddressData::arbitrary(gen));
        }
        ArbitraryAddressDataCollection(addresses)
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
