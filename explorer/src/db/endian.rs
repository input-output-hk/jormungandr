use byteorder::{BigEndian, LittleEndian};
use sanakirja::{direct_repr, Storable, UnsizedStorable};
use zerocopy::{AsBytes, FromBytes, U32, U64};

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsBytes, FromBytes)]
#[repr(transparent)]
pub struct B32(pub U32<BigEndian>);

#[derive(Debug, Clone, PartialEq, Eq, AsBytes, FromBytes)]
#[repr(transparent)]
pub struct L32(pub U32<LittleEndian>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsBytes, FromBytes)]
#[repr(transparent)]
pub struct B64(pub U64<BigEndian>);

#[derive(Debug, Clone, PartialEq, Eq, AsBytes, FromBytes)]
#[repr(transparent)]
pub struct L64(U64<LittleEndian>);

impl L64 {
    pub fn new(n: u64) -> Self {
        Self(U64::<LittleEndian>::new(n))
    }

    pub fn get(&self) -> u64 {
        self.0.get()
    }
}

impl B64 {
    pub fn new(n: u64) -> Self {
        Self(U64::<BigEndian>::new(n))
    }

    pub fn get(&self) -> u64 {
        self.0.get()
    }
}

impl B32 {
    pub fn new(n: u32) -> Self {
        Self(U32::<BigEndian>::new(n))
    }

    pub fn get(&self) -> u32 {
        self.0.get()
    }
}

impl L32 {
    pub fn new(n: u32) -> Self {
        Self(U32::<LittleEndian>::new(n))
    }

    pub fn get(&self) -> u32 {
        self.0.get()
    }
}

impl AsRef<U64<LittleEndian>> for L64 {
    fn as_ref(&self) -> &U64<LittleEndian> {
        &self.0
    }
}

impl Ord for B64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.as_bytes().cmp(other.0.as_bytes())
    }
}

impl PartialOrd for B64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.as_bytes().partial_cmp(other.0.as_bytes())
    }
}

impl Ord for B32 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.as_bytes().cmp(other.0.as_bytes())
    }
}

impl PartialOrd for B32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.as_bytes().partial_cmp(other.0.as_bytes())
    }
}

impl Ord for L64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.get().cmp(&other.0.get())
    }
}

impl PartialOrd for L64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.get().partial_cmp(&other.0.get())
    }
}

impl Ord for L32 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.get().cmp(&other.0.get())
    }
}

impl PartialOrd for L32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.get().partial_cmp(&other.0.get())
    }
}

direct_repr!(B32);
direct_repr!(L32);
direct_repr!(B64);
direct_repr!(L64);
