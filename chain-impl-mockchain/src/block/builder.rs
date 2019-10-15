use super::Block;
use crate::fragment::Contents;
use crate::header::{BlockVersion, Header, HeaderBuilderNew};

/// Create a block from a block version, content and a header builder closure
///
/// If the header builder returns an error, it is returns as is
pub fn builder<E, F>(version: BlockVersion, contents: Contents, hdr_builder: F) -> Result<Block, E>
where
    F: FnOnce(HeaderBuilderNew) -> Result<Header, E>,
{
    hdr_builder(HeaderBuilderNew::new(version, &contents)).map(|header| Block { header, contents })
}
