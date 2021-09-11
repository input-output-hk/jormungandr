use sanakirja::Storable;
use std::fmt;
use zerocopy::{AsBytes, FromBytes};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[repr(C)]
pub struct Pair<A, B> {
    pub a: A,
    pub b: B,
}

impl<A: FromBytes + AsBytes + fmt::Debug, B: FromBytes + AsBytes + fmt::Debug> Storable
    for Pair<A, B>
where
    A: PartialOrd + Ord,
    B: PartialOrd + Ord,
{
    type PageReferences = core::iter::Empty<u64>;
    fn page_references(&self) -> Self::PageReferences {
        core::iter::empty()
    }

    fn compare<T: sanakirja::LoadPage>(&self, _t: &T, b: &Self) -> core::cmp::Ordering {
        (&self.a, &self.b).cmp(&(&b.a, &b.b))
    }
}
