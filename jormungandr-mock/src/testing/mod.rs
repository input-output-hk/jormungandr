pub mod client;
pub mod server;
pub mod setup;

use chain_core::mempack::{ReadBuf, Readable};

pub fn read_into<T: Readable>(bytes: &[u8]) -> T {
    let mut buf = ReadBuf::from(bytes);
    let item = T::read(&mut buf).unwrap();
    item
}
