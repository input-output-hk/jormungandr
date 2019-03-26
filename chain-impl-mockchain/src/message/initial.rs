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

#[derive(Debug, Clone)]
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
