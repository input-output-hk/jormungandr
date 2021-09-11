use sanakirja::{Storable, UnsizedStorable};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[repr(C)]
pub struct Pair<A, B> {
    pub a: A,
    pub b: B,
}

impl<A: Storable, B: Storable> Storable for Pair<A, B> {
    type PageReferences = core::iter::Chain<A::PageReferences, B::PageReferences>;
    fn page_references(&self) -> Self::PageReferences {
        self.a.page_references().chain(self.b.page_references())
    }
    fn compare<T: sanakirja::LoadPage>(&self, t: &T, b: &Self) -> core::cmp::Ordering {
        match self.a.compare(t, &b.a) {
            core::cmp::Ordering::Equal => self.b.compare(t, &b.b),
            ord => ord,
        }
    }
}

impl<A: Ord + UnsizedStorable, B: Ord + UnsizedStorable> UnsizedStorable for Pair<A, B> {
    const ALIGN: usize = std::mem::align_of::<(A, B)>();

    fn size(&self) -> usize {
        let a = self.a.size();
        let b_off = (a + (B::ALIGN - 1)) & !(B::ALIGN - 1);
        (b_off + self.b.size() + (Self::ALIGN - 1)) & !(Self::ALIGN - 1)
    }
    unsafe fn onpage_size(p: *const u8) -> usize {
        let a = A::onpage_size(p);
        let b_off = (a + (B::ALIGN - 1)) & !(B::ALIGN - 1);
        let b_size = B::onpage_size(p.add(b_off));
        (b_off + b_size + (Self::ALIGN - 1)) & !(Self::ALIGN - 1)
    }
    unsafe fn from_raw_ptr<'a, T>(_: &T, p: *const u8) -> &'a Self {
        &*(p as *const Self)
    }
    unsafe fn write_to_page(&self, p: *mut u8) {
        self.a.write_to_page(p);
        let off = (self.a.size() + (B::ALIGN - 1)) & !(B::ALIGN - 1);
        self.b.write_to_page(p.add(off));
    }
}
