use crate::test::Result;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Tag {
    Short,
    Perf,
    Long,
    Feature,
    Unstable,
    All,
}

pub fn parse_tag_from_str(tag: &str) -> Result<Tag> {
    let tag_lowercase: &str = &tag.to_lowercase();
    match tag_lowercase {
        "short" => Ok(Tag::Short),
        "long" => Ok(Tag::Long),
        "perf" => Ok(Tag::Perf),
        "feature" => Ok(Tag::Feature),
        "unstable" => Ok(Tag::Unstable),
        _ => Ok(Tag::All),
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
