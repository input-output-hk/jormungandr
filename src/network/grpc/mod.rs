use blockcfg::chain::cardano::BlockHash;
use cardano::{
    hash,
    util::try_from_slice::TryFromSlice,
};

// Included generated protobuf/gRPC code,
// namespaced into submodules corresponding to the .proto package names

mod cardano {
    include!(concat!(env!("OUT_DIR"), "/cardano.rs"));
}

#[allow(dead_code)]
mod iohk {
    pub mod jormungandr {
        include!(concat!(env!("OUT_DIR"), "/iohk.jormungandr.rs"));
    }
}

mod bootstrap;
mod service;

pub use self::bootstrap::bootstrap_from_peer;
pub use self::service::run_listen_socket;

// Conversions between library data types and their generated
// protobuf counterparts

fn try_hashes_from_protobuf(
    pb: &cardano::HeaderHashes
) -> Result<Vec<BlockHash>, hash::Error> {
    pb.hashes.iter().map(|v| BlockHash::try_from_slice(&v[..])).collect()
}
