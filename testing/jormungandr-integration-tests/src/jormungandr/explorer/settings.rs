use crate::startup;
use chain_impl_mockchain::fee::{PerCertificateFee, PerVoteCertificateFee};
use jormungandr_lib::interfaces::RewardParams;
use jormungandr_lib::interfaces::{Ratio, TaxType};
use chain_impl_mockchain::{block::BlockDate, fee::LinearFee};
use chain_impl_mockchain::fragment::FragmentId;
use chain_impl_mockchain::key::Hash;
use jormungandr_automation::testing::block0::Block0ConfigurationExtension;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Explorer},
};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jortestkit::process::Wait;
use std::num::{NonZeroU64, NonZeroU32};
use std::str::FromStr;
use std::time::Duration;
use thor::{StakePool, TransactionHash};

#[test]
pub fn explorer_config_params() {
    let alice = thor::Wallet::default();

    let mut linear_fees = LinearFee::new(1, 2, 1);
    linear_fees.per_certificate_fees(PerCertificateFee::new(
        NonZeroU64::new(2),
        NonZeroU64::new(3),
        NonZeroU64::new(1),
    ));

    linear_fees.per_vote_certificate_fees(PerVoteCertificateFee::new(
        NonZeroU64::new(3),
        NonZeroU64::new(3),
    ));

    let treasury_parameters = TaxType {
        fixed: 200.into(),
        ratio: Ratio::new_checked(10, 500).unwrap(),
        max_limit: NonZeroU64::new(200),
    };

    let reward_parameters = RewardParams::Linear {
        constant: 500_000,
        ratio: Ratio::new_checked(5, 2_00).unwrap(),
        epoch_start: 2,
        epoch_rate: NonZeroU32::new(4).unwrap(),
    };

    let jormungandr = startup::start_bft(
        vec![&alice],
        ConfigurationBuilder::new()
            .with_linear_fees(linear_fees)
            .with_block_content_max_size(2000.into())
            .with_epoch_stability_depth(2000)
            .with_slot_duration(1)
            .with_slots_per_epoch(6)
            .with_treasury_parameters(treasury_parameters)
            .with_reward_parameters(reward_parameters)
            .with_tx_max_expiry_epochs(50),
    )
    .unwrap();

    let explorer_process = jormungandr.explorer();
    let explorer = explorer_process.client();

    let explorer_settings = explorer.settings().unwrap().data.unwrap().settings;
    println!("{:?}", explorer_settings.fees.coefficient);

    let block0_settings = jormungandr.block0_configuration().settings();

    //assert_eq!(explorer_settings.fees.constant, fee.constant);

}


#[test]
pub fn explorer_settings() {
    let alice = thor::Wallet::default();

    let mut linear_fees = LinearFee::new(1, 2, 1);
    linear_fees.per_certificate_fees(PerCertificateFee::new(
        NonZeroU64::new(2),
        NonZeroU64::new(3),
        NonZeroU64::new(1),
    ));

    linear_fees.per_vote_certificate_fees(PerVoteCertificateFee::new(
        NonZeroU64::new(3),
        NonZeroU64::new(3),
    ));

    let jormungandr = startup::start_bft(
        vec![&alice],
        ConfigurationBuilder::new()
            .with_linear_fees(linear_fees)
            .with_epoch_stability_depth(2000)
    )
    .unwrap();

    let explorer_process = jormungandr.explorer();
    let explorer = explorer_process.client();

    let explorer_settings = explorer.settings().unwrap().data.unwrap().settings;
    let block0_settings = jormungandr.block0_configuration().settings();
    let rest_settings = jormungandr.rest().settings().unwrap();
    println!("Explorer- {:?} \n Block0- {:?} \n Rest- {:?}", explorer_settings.fees, block0_settings.fees, rest_settings.fees);


    //assert_eq!(block0_settings.fees, explorer_settings.fees.into());
}