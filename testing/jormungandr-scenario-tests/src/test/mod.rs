pub mod comm;
pub mod features;
pub mod legacy;
pub mod network;
pub mod non_functional;
pub mod utils;

pub type Result<T> = ::core::result::Result<T, hersir::error::Error>;
