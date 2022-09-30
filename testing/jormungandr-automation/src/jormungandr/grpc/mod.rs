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

use chain_core::{packer::Codec, property::DeserializeFromSlice};

pub fn read_into<T: DeserializeFromSlice>(bytes: &[u8]) -> T {
    let mut buf = Codec::new(bytes);
    T::deserialize_from_slice(&mut buf).unwrap()
}
