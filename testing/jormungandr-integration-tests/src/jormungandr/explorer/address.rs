use crate::startup;
use chain_impl_mockchain::fragment::FragmentId;
use chain_impl_mockchain::key::Hash;
use chain_impl_mockchain::{block::BlockDate, transaction};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Explorer},
};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jortestkit::process::Wait;
use std::time::Duration;
use std::{borrow::Borrow, str::FromStr};
use thor::TransactionHash;

#[test]
pub fn explorer_address_test() {
    let jcli: JCli = Default::default();
    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let transaction_value = 1_000;

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _initial_stake_pools) =
        startup::start_stake_pool(&[sender.clone()], &[], &mut config).unwrap();

    let explorer_process = jormungandr.explorer();
    let explorer = explorer_process.client();

    explorer
        .address(sender.address_bech32(chain_addr::Discrimination::Test))
        .unwrap();
}