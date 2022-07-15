use assert_fs::TempDir;
use chain_impl_mockchain::{chaintypes::ConsensusType, testing::WitnessMode};
use jormungandr_automation::jormungandr::{ConfigurationBuilder, Starter};
use jormungandr_lib::interfaces::InitialUTxO;
use thor::{FragmentSender, FragmentSenderSetup, FragmentVerifier};

#[test]
fn parallel_transaction_using_different_lanes() {
    let temp_dir = TempDir::new().unwrap();
    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let config = ConfigurationBuilder::new()
        .with_slots_per_epoch(20)
        .with_slot_duration(2)
        .with_block0_consensus(ConsensusType::Bft)
        .with_funds(vec![
            InitialUTxO {
                address: sender.address(),
                value: 100_000.into(),
            },
            InitialUTxO {
                address: receiver.address(),
                value: 100_000.into(),
            },
        ])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .config(config)
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let mut fragment_sender = FragmentSender::from_with_setup(
        jormungandr.block0_configuration(),
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
