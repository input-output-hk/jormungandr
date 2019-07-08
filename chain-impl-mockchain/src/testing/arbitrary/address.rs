use crate::testing::{address::AddressData, arbitrary::kind_type::KindTypeWithoutMultisig};
use chain_addr::Discrimination;
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
