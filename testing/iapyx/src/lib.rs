extern crate rand;
#[macro_use(lazy_static)]
extern crate lazy_static;

mod backend;
pub mod cli;
mod controller;
mod data;
mod load;
pub mod utils;
mod wallet;

pub use crate::wallet::{Wallet, WalletError};
pub use backend::WalletBackend;
pub use controller::Controller;
pub use data::{Fund, Proposal, SimpleVoteStatus, Voteplan};
pub use load::{MultiController, VoteStatusProvider, WalletRequestGen};
