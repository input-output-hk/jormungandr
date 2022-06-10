use crate::startup;
use chain_impl_mockchain::{block::BlockDate, fee::LinearFee};
use chain_impl_mockchain::fragment::FragmentId;
use chain_impl_mockchain::key::Hash;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Explorer},
};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jortestkit::process::Wait;
use std::str::FromStr;
use std::time::Duration;
use thor::{StakePool, TransactionHash};

#[test]
pub fn explorer_settings_test() {
    let jcli: JCli = Default::default();
    let faucet = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let fee = LinearFee::new(5, 5, 5);

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM).with_linear_fees(fee);

    let (jormungandr, initial_stake_pools) =
        startup::start_stake_pool(&[faucet.clone()], &[], &mut config).unwrap();

    let explorer_process = jormungandr.explorer();
    let explorer = explorer_process.client();

    let explorer_settings = explorer.settings().unwrap().data.unwrap().settings;
    println!("{:?}", explorer_settings );
    //assert_eq!(explorer_settings.fees.per_vote_certificate_fees, fee.constant);

}
