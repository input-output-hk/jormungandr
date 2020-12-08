use crate::common::jormungandr::ConfigurationBuilder;
use crate::common::startup;
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, KESUpdateSpeed};
use jormungandr_testing_utils::testing::{
    BatchFragmentGenerator, FragmentGenerator, FragmentSenderSetup, FragmentStatusProvider,
};
pub use jortestkit::console::progress_bar::{parse_progress_bar_mode_from_str, ProgressBarMode};
use jortestkit::load::{self, Configuration, Monitor};

#[test]
pub fn fragment_load_test() {
    let mut faucet = startup::create_new_account_address();

    let (mut jormungandr, _) = startup::start_stake_pool(
        &[faucet.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_epoch_stability_depth(10)
            .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    jormungandr.steal_temp_dir().unwrap().into_persistent();

    let configuration = Configuration::duration(
        10,
        std::time::Duration::from_secs(60),
        100,
        Monitor::Standard(100),
        0,
    );

    let mut request_generator = FragmentGenerator::new(
        FragmentSenderSetup::no_verify(),
        jormungandr.to_remote(),
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
    );
    request_generator.fill_from_faucet(&mut faucet);

    load::start_async(
        request_generator,
        FragmentStatusProvider::new(jormungandr.to_remote()),
        configuration,
        "Wallet backend load test",
    );
}

#[test]
pub fn fragment_batch_load_test() {
    let mut faucet = startup::create_new_account_address();

    let (mut jormungandr, _) = startup::start_stake_pool(
        &[faucet.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_epoch_stability_depth(10)
            .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    jormungandr.steal_temp_dir().unwrap().into_persistent();

    let configuration = Configuration::duration(
        5,
        std::time::Duration::from_secs(60),
        1000,
        Monitor::Standard(100),
        0,
    );

    let mut request_generator = BatchFragmentGenerator::new(
        FragmentSenderSetup::no_verify(),
        jormungandr.to_remote(),
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        10,
    );
    request_generator.fill_from_faucet(&mut faucet);

    load::start_async(
        request_generator,
        FragmentStatusProvider::new(jormungandr.to_remote()),
        configuration,
        "Wallet backend load test",
    );
}
