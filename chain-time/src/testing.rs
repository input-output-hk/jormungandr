use crate::{Epoch, Slot, TimeEra};
use quickcheck::{Arbitrary, Gen};

impl Arbitrary for TimeEra {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        TimeEra::new(
            Arbitrary::arbitrary(g),
            Arbitrary::arbitrary(g),
            u32::arbitrary(g) % 127 + 1,
        )
    }
}

impl Arbitrary for Slot {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Slot(Arbitrary::arbitrary(g))
    }
}

impl Arbitrary for Epoch {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Epoch(Arbitrary::arbitrary(g))
    }
}
