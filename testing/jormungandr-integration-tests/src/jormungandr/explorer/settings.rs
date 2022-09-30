use crate::startup;
use chain_impl_mockchain::fee::{LinearFee, PerCertificateFee, PerVoteCertificateFee};
use jormungandr_automation::jormungandr::{
    explorer::{configuration::ExplorerParams, verifiers::ExplorerVerifier},
    ConfigurationBuilder,
};
use jormungandr_lib::interfaces::DEFAULT_EPOCH_STABILITY_DEPTH;
use std::num::NonZeroU64;

#[test]
#[should_panic] //BUG -> NPG-2098

pub fn explorer_settings() {
    let alice = thor::Wallet::default();
    let constant_fee = 1;
    let coefficient_fee = 2;
    let certificate_fee = 1;

    let mut linear_fees = LinearFee::new(constant_fee, coefficient_fee, certificate_fee);
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
            .with_linear_fees(linear_fees.clone())
            .with_epoch_stability_depth(DEFAULT_EPOCH_STABILITY_DEPTH),
    )
    .unwrap();

    let explorer_process = jormungandr.explorer(ExplorerParams::default()).unwrap();
    let explorer = explorer_process.client();
    let explorer_settings = explorer.settings().unwrap().data.unwrap().settings;

    ExplorerVerifier::assert_fees(linear_fees, explorer_settings.fees);
    ExplorerVerifier::assert_epoch_stability_depth(
        DEFAULT_EPOCH_STABILITY_DEPTH,
        explorer_settings
            .epoch_stability_depth
            .epoch_stability_depth,
    );
}
