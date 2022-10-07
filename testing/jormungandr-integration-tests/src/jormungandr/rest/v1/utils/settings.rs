use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use chain_impl_mockchain::fee::{LinearFee, PerCertificateFee, PerVoteCertificateFee};
use jormungandr_automation::{
    jormungandr::Block0ConfigurationBuilder, testing::block0::Block0ConfigurationExtension,
};
use jormungandr_lib::interfaces::{Ratio, RewardParams, TaxType};
use std::num::{NonZeroU32, NonZeroU64};
use thor::Block0ConfigurationBuilderExtension;

#[test]
pub fn test_default_settings() {
    let temp_dir = TempDir::new().unwrap();

    let config_builder = Block0ConfigurationBuilder::default();

    let test_context = SingleNodeTestBootstrapper::default()
        .with_block0_config(config_builder)
        .as_bft_leader()
        .build();
    let jormungandr = test_context.start_node(temp_dir).unwrap();
    let config = test_context.block0_config();
    let rest_settings = jormungandr.rest().settings().expect("Rest settings error");
    let block0_settings = config.settings();
    assert_eq!(rest_settings, block0_settings);
}

#[test]
pub fn test_custom_settings() {
    let temp_dir = TempDir::new().unwrap();

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

    let block0_config = Block0ConfigurationBuilder::default()
        .with_linear_fees(linear_fees)
        .with_block_content_max_size(2000.into())
        .with_epoch_stability_depth(2000.try_into().unwrap())
        .with_slot_duration(1.try_into().unwrap())
        .with_slots_per_epoch(6.try_into().unwrap())
        .with_treasury_parameters(Some(treasury_parameters))
        .with_reward_parameters(Some(reward_parameters))
        .with_tx_max_expiry_epochs(50)
        .with_wallet(&alice, 100.into());

    let test_context = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(block0_config)
        .build();

    let jormungandr = test_context.start_node(temp_dir).unwrap();

    let rest_settings = jormungandr.rest().settings().unwrap();
    let block0_settings = test_context.block0_config().settings();
    assert_eq!(rest_settings, block0_settings);
}
