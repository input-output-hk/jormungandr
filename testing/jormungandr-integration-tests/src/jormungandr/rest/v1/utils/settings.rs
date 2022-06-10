use crate::startup;
use chain_impl_mockchain::fee::{LinearFee, PerCertificateFee, PerVoteCertificateFee};
use jormungandr_automation::{
    jormungandr::ConfigurationBuilder, testing::block0::Block0ConfigurationExtension,
};
use jormungandr_lib::interfaces::{Ratio, RewardParams, TaxType};
use std::num::{NonZeroU32, NonZeroU64};

#[test]
pub fn test_default_settings() {
    let alice = thor::Wallet::default();
    let bob = thor::Wallet::default();
    let (jormungandr, _stake_pools) =
        startup::start_stake_pool(&[alice], &[bob], &mut ConfigurationBuilder::new()).unwrap();

    let rest_settings = jormungandr.rest().settings().expect("Rest settings error");
    let block0_settings = jormungandr.block0_configuration().settings();
    assert_eq!(rest_settings, block0_settings);
}

#[test]
pub fn test_custom_settings() {
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

    let rest_settings = jormungandr.rest().settings().unwrap();
    let block0_settings = jormungandr.block0_configuration().settings();
    assert_eq!(rest_settings, block0_settings);
}
