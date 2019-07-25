#[macro_use]
extern crate cfg_if;

cfg_if! {
    if #[cfg(test)] {
        extern crate quickcheck;
    } else if #[cfg(feature = "property-test-api")] {
        extern crate quickcheck;
    }
}

pub mod abor;
pub mod mempack;
pub mod packer;
pub mod property;
