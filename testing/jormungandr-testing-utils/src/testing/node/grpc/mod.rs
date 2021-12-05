pub mod client;
pub mod server;

pub use client::JormungandrClient;
pub use server::JormungandrServerImpl;

mod node {
    tonic::include_proto!("iohk.chain.node"); // The string specified here must match the proto package name
}

mod types {
    tonic::include_proto!("iohk.chain.types"); // The string specified here must match the proto package name
}

mod watch {
    tonic::include_proto!("iohk.chain.watch"); // The string specified here must match the proto package name
}

use chain_core::mempack::{ReadBuf, Readable};

pub fn read_into<T: Readable>(bytes: &[u8]) -> T {
    let mut buf = ReadBuf::from(bytes);
    T::read(&mut buf).unwrap()
}
