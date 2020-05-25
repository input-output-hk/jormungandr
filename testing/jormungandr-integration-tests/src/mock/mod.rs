#[macro_use]
pub mod client;
pub mod server;
#[cfg(test)]
pub mod testing;

pub use client::JormungandrClient;
pub use server::JormungandrServerImpl;

use chain_core::mempack::{ReadBuf, Readable};

pub fn read_into<T: Readable>(bytes: &[u8]) -> T {
    let mut buf = ReadBuf::from(bytes);
    let item = T::read(&mut buf).unwrap();
    item
}
