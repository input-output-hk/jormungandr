use chain_addr::{Address, Discrimination, Kind, KindType};
use chain_impl_mockchain::transaction::Output;
use quickcheck::{Arbitrary, Gen};

#[derive(Clone, Debug)]
pub struct KindTypeWithoutMultisig(pub KindType);

impl Arbitrary for KindTypeWithoutMultisig {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut kind_type: KindType;
        loop {
            kind_type = KindType::arbitrary(g);
            if kind_type != KindType::Multisig {
                break;
            }
        }
        KindTypeWithoutMultisig(kind_type)
    }
}

impl KindTypeWithoutMultisig {
    pub fn kind_type(&self) -> KindType {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct KindWithoutMultisig(pub Kind);

impl Arbitrary for KindWithoutMultisig {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut kind: Kind;
        loop {
            kind = Kind::arbitrary(g);
            match kind {
                Kind::Multisig { .. } => (),
                _ => {
                    break;
                }
            }
        }
        KindWithoutMultisig(kind)
    }
}
