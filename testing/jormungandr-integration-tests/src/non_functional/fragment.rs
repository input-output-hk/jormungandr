use crate::common::jormungandr::ConfigurationBuilder;
use crate::common::startup;
use chain_impl_mockchain::block::BlockDate;
use jormungandr_lib::interfaces::BlockDate as BlockDateDto;
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, KesUpdateSpeed};
use jormungandr_testing_utils::testing::fragments::TransactionGenerator;
use jormungandr_testing_utils::testing::node::time;
use jormungandr_testing_utils::testing::FragmentSender;
use jormungandr_testing_utils::testing::{
    BatchFragmentGenerator, FragmentGenerator, FragmentSenderSetup, FragmentStatusProvider,
};
pub use jortestkit::console::progress_bar::{parse_progress_bar_mode_from_str, ProgressBarMode};
use jortestkit::load::{self, Configuration, Monitor};
use jortestkit::prelude::Wait;
use std::time::Duration;

#[test]
pub fn fragment_load_test() {
    let faucet = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();

    let (mut jormungandr, _) = startup::start_stake_pool(
        &[faucet.clone()],
        &[receiver.clone()],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(30)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_epoch_stability_depth(10)
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    jormungandr.steal_temp_dir().unwrap().into_persistent();

    let configuration = Configuration::duration(
        1,
        std::time::Duration::from_secs(60),
        500,
        Monitor::Standard(1000),
        1_000,
        1_000,
    );

    let mut request_generator = FragmentGenerator::new(
        faucet,
        receiver,
        jormungandr.to_remote(),
        60,
        30,
        30,
        30,
        FragmentSender::new(
            jormungandr.genesis_block_hash(),
            jormungandr.fees(),
            BlockDate::first().next_epoch().into(),
            FragmentSenderSetup::no_verify(),
        ),
    );

    use crate::common::jcli::FragmentsCheck;
    use crate::common::jcli::JCli;

    request_generator.prepare(BlockDateDto::new(0, 19));

    let jcli: JCli = Default::default();

    let fragment_check = FragmentsCheck::new(jcli, &jormungandr);
    let wait = Wait::new(Duration::from_secs(1), 25);
    fragment_check.wait_until_all_processed(&wait).unwrap();

    time::wait_for_epoch(1, jormungandr.rest());

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
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    jormungandr.steal_temp_dir().unwrap().into_persistent();

    let configuration = Configuration::duration(
        5,
        std::time::Duration::from_secs(60),
        1000,
        Monitor::Standard(100),
        3_000,
        1_000,
    );

    let mut request_generator = BatchFragmentGenerator::new(
        FragmentSenderSetup::no_verify(),
        jormungandr.to_remote(),
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDate::first().into(),
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

#[test]
pub fn transaction_load_test() {
    let mut faucet = startup::create_new_account_address();

    let (mut jormungandr, _) = startup::start_stake_pool(
        &[faucet.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_epoch_stability_depth(10)
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    jormungandr.steal_temp_dir().unwrap().into_persistent();

    let configuration = Configuration::duration(
        1,
        std::time::Duration::from_secs(60),
        100,
        Monitor::Standard(100),
        100,
        1_000,
    );

    let mut request_generator = TransactionGenerator::new(
        FragmentSenderSetup::no_verify(),
        jormungandr.to_remote(),
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDate::first().next_epoch().into(),
    );
    request_generator.fill_from_faucet(&mut faucet);

    load::start_async(
        request_generator,
        FragmentStatusProvider::new(jormungandr.to_remote()),
        configuration,
        "Wallet backend load test",
    );
}
