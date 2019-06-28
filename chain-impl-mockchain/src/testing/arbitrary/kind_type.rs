use chain_addr::{Kind, KindType};
use quickcheck::{Arbitrary, Gen};
use std::iter;

#[derive(Clone, Debug)]
pub struct KindTypeWithoutMultisig(pub KindType);

impl Arbitrary for KindTypeWithoutMultisig {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        KindTypeWithoutMultisig(
            iter::from_fn(|| Some(KindType::arbitrary(g)))
                .filter(|x| match x {
                    KindType::Multisig => false,
                    _ => true,
                })
                .next()
                .unwrap(),
        )
    }
}

impl KindTypeWithoutMultisig {
    pub fn kind_type(&self) -> KindType {
        self.0
    }
}

impl From<KindTypeWithoutMultisig> for KindType {
    fn from(kind_type_without_multisig: KindTypeWithoutMultisig) -> Self {
        kind_type_without_multisig.kind_type()
    }
}

#[derive(Clone, Debug)]
pub struct KindWithoutMultisig(pub Kind);

impl Arbitrary for KindWithoutMultisig {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        KindWithoutMultisig(
            iter::from_fn(|| Some(Kind::arbitrary(g)))
                .filter(|x| match x {
                    Kind::Multisig { .. } => false,
                    _ => true,
                })
                .next()
                .unwrap(),
        )
    }
}
