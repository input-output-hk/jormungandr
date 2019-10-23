extern crate base64;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate hex;
extern crate protobuf;

#[macro_use]
pub mod client;
pub mod convert;
pub mod proto;
pub mod server;

use chain_core::mempack::{ReadBuf, Readable};
pub use client::JormungandrClient;
pub use convert::*;
pub use server::JormungandrServerImpl;

pub fn read_into<T: Readable>(bytes: &[u8]) -> T {
    let mut buf = ReadBuf::from(bytes);
    let item = T::read(&mut buf).unwrap();
    item
}
