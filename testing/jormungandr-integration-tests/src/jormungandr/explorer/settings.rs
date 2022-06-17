use crate::startup;
use chain_impl_mockchain::fee::LinearFee;
use chain_impl_mockchain::fee::{PerCertificateFee, PerVoteCertificateFee};
use jormungandr_automation::jormungandr::ConfigurationBuilder;
use jormungandr_automation::testing::block0::Block0ConfigurationExtension;
use std::num::NonZeroU64;

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
            .with_epoch_stability_depth(2000),
    )
    .unwrap();

    let explorer_process = jormungandr.explorer();
    let explorer = explorer_process.client();

    let explorer_settings = explorer.settings().unwrap().data.unwrap().settings;
    let block0_settings = jormungandr.block0_configuration().settings();

    assert_eq!(
        explorer_settings.fees.certificate as u64,
        block0_settings.fees.certificate
    );
    assert_eq!(
        explorer_settings.fees.coefficient as u64,
        block0_settings.fees.coefficient
    );
    assert_eq!(
        explorer_settings.fees.constant as u64,
        block0_settings.fees.constant
    );
    assert_eq!(
        explorer_settings
            .fees
            .per_certificate_fees
            .certificate_owner_stake_delegation
            .unwrap() as u64,
        u64::from(
            block0_settings
                .fees
                .per_certificate_fees
                .certificate_owner_stake_delegation
                .unwrap()
        )
    );
    assert_eq!(
        explorer_settings
            .fees
            .per_certificate_fees
            .certificate_pool_registration
            .unwrap() as u64,
        u64::from(
            block0_settings
                .fees
                .per_certificate_fees
                .certificate_pool_registration
                .unwrap()
        )
    );
    assert_eq!(
        explorer_settings
            .fees
            .per_certificate_fees
            .certificate_stake_delegation
            .unwrap() as u64,
        u64::from(
            block0_settings
                .fees
                .per_certificate_fees
                .certificate_stake_delegation
                .unwrap()
        )
    );
    assert_eq!(
        explorer_settings
            .fees
            .per_vote_certificate_fees
            .certificate_vote_cast
            .unwrap() as u64,
        u64::from(
            block0_settings
                .fees
                .per_vote_certificate_fees
                .certificate_vote_cast
                .unwrap()
        )
    );
    assert_eq!(
        explorer_settings
            .fees
            .per_vote_certificate_fees
            .certificate_vote_plan
            .unwrap() as u64,
        u64::from(
            block0_settings
                .fees
                .per_vote_certificate_fees
                .certificate_vote_plan
                .unwrap()
        )
    );
}
