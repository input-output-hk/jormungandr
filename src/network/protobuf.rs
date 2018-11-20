pub mod cardano {
    include!(concat!(env!("OUT_DIR"), "/cardano.rs"));
}

pub mod iohk {
    pub mod jormungandr {
        include!(concat!(env!("OUT_DIR"), "/iohk.jormungandr.rs"));
    }
}