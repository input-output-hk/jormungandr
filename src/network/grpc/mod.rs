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
