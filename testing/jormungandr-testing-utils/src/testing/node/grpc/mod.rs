pub mod client;
pub mod server;

pub use client::JormungandrClient;
pub use server::JormungandrServerImpl;

mod proto {
    tonic::include_proto!("iohk.chain.node"); // The string specified here must match the proto package name
}

use chain_core::mempack::{ReadBuf, Readable};

pub fn read_into<T: Readable>(bytes: &[u8]) -> T {
    let mut buf = ReadBuf::from(bytes);
    T::read(&mut buf).unwrap()
}
