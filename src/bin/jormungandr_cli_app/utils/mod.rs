#![allow(dead_code)]

mod host_addr;
mod segment_parser;
pub mod serde_with_string;

pub use self::host_addr::HostAddr;
pub use self::segment_parser::SegmentParser;
