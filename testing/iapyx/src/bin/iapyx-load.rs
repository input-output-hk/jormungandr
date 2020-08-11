use chain_addr::{AddressReadable, Discrimination};
use iapyx::cli::args::load::IapyxLoadCommand;
use iapyx::{
    cli::args::interactive::UserInteraction, Controller, MultiController, SimpleVoteStatus,
    VoteStatusProvider, WalletRequestGen,
};
use jormungandr_lib::interfaces::{AccountIdentifier, Address};
use jortestkit::load;
use load::{Configuration, Monitor};
use std::str;
use std::str::FromStr;
use structopt::StructOpt;
use thiserror::Error;
use wallet_core::Choice;

pub fn main() {
    IapyxLoadCommand::from_args().exec().unwrap();
}
