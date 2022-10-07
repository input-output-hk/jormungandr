use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use chain_impl_mockchain::testing::WitnessMode;
use jormungandr_automation::jormungandr::Block0ConfigurationBuilder;
use jormungandr_lib::interfaces::{NumberOfSlotsPerEpoch, SlotDuration};
use thor::{FragmentSender, FragmentSenderSetup, FragmentVerifier};

#[test]
fn parallel_transaction_using_different_lanes() {
    let temp_dir = TempDir::new().unwrap();
    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let config = Block0ConfigurationBuilder::default()
        .with_slots_per_epoch(NumberOfSlotsPerEpoch::new(20).unwrap())
        .with_slot_duration(SlotDuration::new(2).unwrap())
        .with_utxos(vec![
            sender.to_initial_fund(100_000),
            receiver.to_initial_fund(100_000),
        ]);

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(config)
        .build()
        .start_node(temp_dir)
        .unwrap();

    let mut fragment_sender = FragmentSender::from_settings_with_setup(
        &jormungandr.rest().settings().unwrap(),
        FragmentSenderSetup::no_verify(),
    );

    let mut checks = vec![];

    fragment_sender = fragment_sender.witness_mode(WitnessMode::Account { lane: 1 });
    checks.push(
        fragment_sender
            .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
            .unwrap(),
    );

    fragment_sender = fragment_sender.witness_mode(WitnessMode::Account { lane: 2 });
    checks.push(
        fragment_sender
            .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
            .unwrap(),
    );

    fragment_sender = fragment_sender.witness_mode(WitnessMode::Account { lane: 3 });
    checks.push(
        fragment_sender
            .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
            .unwrap(),
    );

    fragment_sender = fragment_sender.witness_mode(WitnessMode::Account { lane: 4 });
    checks.push(
        fragment_sender
            .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
            .unwrap(),
    );

    FragmentVerifier::wait_and_verify_all_are_in_block(
        std::time::Duration::from_secs(10),
        checks,
        &jormungandr,
    )
    .unwrap();
}
