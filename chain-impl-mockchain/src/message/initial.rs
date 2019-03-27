use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tag(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TagLen(u16);

pub type TagPayload = Vec<u8>;

const MAXIMUM_TAG: u16 = 1024;
const MAXIMUM_LEN: usize = 64;

impl Tag {
    pub fn new(tag: u16) -> Option<Self> {
        if tag < MAXIMUM_TAG {
            Some(Tag(tag))
        } else {
            None
        }
    }

    pub(crate) const fn unchecked_new(tag: u16) -> Self {
        Tag(tag)
    }
}

impl TagLen {
    pub fn new(tag: Tag, len: usize) -> Option<Self> {
        if len < MAXIMUM_LEN {
            Some(TagLen(tag.0 << 6 | len as u16))
        } else {
            None
        }
    }

    pub fn deconstruct(self) -> (Tag, usize) {
        (self.get_tag(), self.get_len())
    }

    pub fn get_tag(self) -> Tag {
        Tag(self.0 >> 6)
    }

    pub fn get_len(self) -> usize {
        (self.0 & 0b11_1111) as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialEnts(Vec<(Tag, TagPayload)>);

impl InitialEnts {
    pub fn new() -> Self {
        InitialEnts(Vec::new())
    }

    pub fn push(&mut self, t: (Tag, TagPayload)) {
        self.0.push(t)
    }

    pub fn iter(&self) -> std::slice::Iter<(Tag, TagPayload)> {
        self.0.iter()
    }
}

impl property::Serialize for InitialEnts {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::Codec;
        use std::io::Write;
        let mut codec = Codec::from(writer);
        for (tag, bytes) in self.iter() {
            match TagLen::new(*tag, bytes.len()) {
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "initial ent payload too big".to_owned(),
                    ));
                }
                Some(taglen) => {
                    codec.put_u16(taglen.0)?;
                    codec.write_all(&bytes)?;
                }
            };
        }
        Ok(())
    }
}

impl Readable for InitialEnts {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let mut ents = Vec::new();

        while !(buf.is_end()) {
            let taglen = TagLen(buf.get_u16()?);
            let (tag, len) = taglen.deconstruct();
            let mut bytes = vec![0u8; len];
            bytes.extend_from_slice(buf.get_slice(len)?);
            ents.push((tag, bytes))
        }
        Ok(InitialEnts(ents))
    }
}

pub const TAG_DISCRIMINATION: Tag = Tag(1);
pub const TAG_BLOCK0_DATE: Tag = Tag(2);

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::{entity_to, Block0Date};
    use quickcheck::{Arbitrary, Gen, TestResult};

    quickcheck! {
        fn initial_ents_serialization_bijection(b: InitialEnts) -> TestResult {
            property::testing::serialization_bijection_r(b)
        }

        fn tag_len_computation_correct(b: InitialEnt) -> TestResult {
            let InitialEnt(tag, payload) = b;
            let tag_len = if let Some(tg)= TagLen::new(tag, payload.len()) {
                tg
            } else {
                return TestResult::error(format!("cannot construct valid TagLen"));
            };

            let (tag_, len_) = tag_len.deconstruct();

            if tag_ != tag {
                return TestResult::error(format!("Invalid decoded Tag, received: {:?}", tag_));
            }
            if len_ != payload.len() {
                return TestResult::error(format!("Invalid decoded Len: received: {}", len_));
            }
            TestResult::passed()
        }
    }

    fn arbitrary_discrimination<G: Gen>(g: &mut G) -> (Tag, TagPayload) {
        match u8::arbitrary(g) % 2 {
            0 => entity_to(&chain_addr::Discrimination::Production),
            _ => entity_to(&chain_addr::Discrimination::Test),
        }
    }

    fn arbitrary_tag_payload<G: Gen>(g: &mut G) -> (Tag, TagPayload) {
        match u8::arbitrary(g) % 2 {
            0 => arbitrary_discrimination(g),
            _ => entity_to(&Block0Date::arbitrary(g)),
        }
    }

    #[derive(PartialEq, Eq, Clone, Debug)]
    struct InitialEnt(Tag, TagPayload);

    impl Arbitrary for InitialEnt {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let (tag, payload) = arbitrary_tag_payload(g);
            InitialEnt(tag, payload)
        }
    }

    impl Arbitrary for InitialEnts {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let size = u8::arbitrary(g) as usize;
            InitialEnts(
                std::iter::repeat_with(move || arbitrary_tag_payload(g))
                    .take(size)
                    .collect(),
            )
        }
    }
}
