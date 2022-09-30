use crate::startup;
use chain_impl_mockchain::{block::BlockDate, fee::LinearFee, fragment::Fragment};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::ConfigurationBuilder,
    testing::{
        benchmark_efficiency, benchmark_endurance, benchmark_speed, time, EfficiencyBenchmarkDef,
        EfficiencyBenchmarkFinish, Endurance, Thresholds,
    },
};
use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, BlockDate as JLibBlockDate, KesUpdateSpeed, Mempool,
};
use std::{iter, time::Duration};
use thor::{BlockDateGenerator, FragmentSender, FragmentSenderSetup, Wallet};

#[test]
pub fn test_100_transaction_is_processed_in_10_packs_to_many_accounts() {
    let receivers: Vec<Wallet> = iter::from_fn(|| Some(thor::Wallet::default()))
        .take(10)
        .collect();
    send_and_measure_100_transaction_in_10_packs_for_recievers(
        receivers,
        "100_transaction_are_processed_in_10_packs_to_many_accounts",
    );
}

#[test]
pub fn test_100_transaction_is_processed_in_10_packs_to_single_account() {
    let single_reciever = thor::Wallet::default();
    let receivers: Vec<Wallet> = iter::from_fn(|| Some(single_reciever.clone()))
        .take(10)
        .collect();
    send_and_measure_100_transaction_in_10_packs_for_recievers(
        receivers,
        "100_transaction_are_processed_in_10_packs_to_single_account",
    );
}

fn send_and_measure_100_transaction_in_10_packs_for_recievers(receivers: Vec<Wallet>, info: &str) {
    let pack_size = 10;
    let target = (pack_size * receivers.len()) as u32;
    let efficiency_benchmark_result = send_100_transaction_in_10_packs_for_recievers(
        pack_size,
        receivers,
        benchmark_efficiency(info.to_owned()).target(target),
    );
    efficiency_benchmark_result.print();
}

fn send_100_transaction_in_10_packs_for_recievers(
    iterations_count: usize,
    receivers: Vec<Wallet>,
    efficiency_benchmark_def: &mut EfficiencyBenchmarkDef,
) -> EfficiencyBenchmarkFinish {
    let mut sender = thor::Wallet::default();
    let jcli: JCli = Default::default();
    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(2)
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    let settings = jormungandr.rest().settings().unwrap();
    let block_date_generator = BlockDateGenerator::rolling(
        &settings,
        BlockDate {
            epoch: 1,
            slot_id: 0,
        },
        false,
    );
    let output_value = 1_u64;
    let mut efficiency_benchmark_run = efficiency_benchmark_def.start();
    for i in 0..iterations_count {
        let transation_messages: Vec<String> = receivers
            .iter()
            .map(|receiver| {
                let message = jcli
                    .transaction_builder(jormungandr.genesis_block_hash())
                    .new_transaction()
                    .add_account(&sender.address().to_string(), &output_value.into())
                    .add_output(&receiver.address().to_string(), output_value.into())
                    .set_expiry_date(block_date_generator.block_date().into())
                    .finalize()
                    .seal_with_witness_data(sender.witness_data())
                    .to_message();
                sender.confirm_transaction();
                message
            })
            .collect();

        println!("Sending pack of 10 transaction no. {}", i);
        if let Err(err) = super::send_transaction_and_ensure_block_was_produced(
            &transation_messages,
            &jormungandr,
        ) {
            return efficiency_benchmark_run.exception(err.to_string());
        }

        efficiency_benchmark_run.increment_by(receivers.len() as u32);
    }
    efficiency_benchmark_run.stop()
}

#[test]
pub fn test_100_transaction_is_processed_simple() {
    let transaction_max_count = 100;
    let mut sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let jcli: JCli = Default::default();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    let settings = jormungandr.rest().settings().unwrap();
    let block_date_generator = BlockDateGenerator::rolling(
        &settings,
        BlockDate {
            epoch: 1,
            slot_id: 0,
        },
        false,
    );
    let output_value = 1_u64;
    let mut benchmark = benchmark_efficiency("test_100_transaction_is_processed_simple")
        .target(transaction_max_count)
        .start();

    for i in 0..transaction_max_count {
        let transaction = jcli
            .transaction_builder(jormungandr.genesis_block_hash())
            .new_transaction()
            .add_account(&sender.address().to_string(), &output_value.into())
            .add_output(&receiver.address().to_string(), output_value.into())
            .set_expiry_date(block_date_generator.block_date().into())
            .finalize()
            .seal_with_witness_data(sender.witness_data())
            .to_message();

        sender.confirm_transaction();
        println!("Sending transaction no. {}", i + 1);

        if let Err(error) = super::check_transaction_was_processed(
            transaction.to_owned(),
            &receiver,
            (i + 1).into(),
            &jormungandr,
        ) {
            let message = format!("{}", error);
            benchmark.exception(message).print();
            return;
        }

        benchmark.increment();
    }
    benchmark.stop().print();
    jcli.fragments_checker(&jormungandr)
        .check_log_shows_in_block()
        .expect("cannot read logs");
}

#[test]
pub fn test_blocks_are_being_created_for_more_than_15_minutes() {
    let mut sender = thor::Wallet::default();
    let mut receiver = thor::Wallet::default();
    let jcli: JCli = Default::default();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_epoch_stability_depth(10)
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    let output_value = 1_u64;
    let benchmark = benchmark_endurance("test_blocks_are_created_for_more_than_15_minutes")
        .target(Duration::from_secs(900))
        .start();

    let settings = jormungandr.rest().settings().unwrap();
    let block_date_generator = BlockDateGenerator::rolling(
        &settings,
        BlockDate {
            epoch: 1,
            slot_id: 0,
        },
        false,
    );

    loop {
        let transaction = jcli
            .transaction_builder(jormungandr.genesis_block_hash())
            .new_transaction()
            .add_account(&sender.address().to_string(), &output_value.into())
            .add_output(&receiver.address().to_string(), output_value.into())
            .set_expiry_date(block_date_generator.block_date().into())
            .finalize()
            .seal_with_witness_data(sender.witness_data())
            .to_message();

        sender.confirm_transaction();
        if let Err(err) =
            super::send_transaction_and_ensure_block_was_produced(&[transaction], &jormungandr)
        {
            let error_message = format!("{:?}", err);
            // temporary threshold for the time issue with transaction stuck is resolved
            let temporary_threshold =
                Thresholds::<Endurance>::new_endurance(Duration::from_secs(400));
            benchmark
                .exception(error_message)
                .print_with_thresholds(temporary_threshold);
            return;
        }

        if benchmark.max_endurance_reached() {
            benchmark.stop().print();
            return;
        }

        std::mem::swap(&mut sender, &mut receiver);
    }
}

#[test]
pub fn test_expired_transactions_processing_speed() {
    const N_TRANSACTIONS: usize = 100_000;

    let mut sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    time::wait_for_date(JLibBlockDate::new(0, 1), jormungandr.rest());

    let output_value = 1;
    let transactions: Vec<Fragment> = (0..N_TRANSACTIONS)
        .map(|_| {
            let tx = thor::FragmentBuilder::new(
                &jormungandr.genesis_block_hash(),
                &jormungandr.fees(),
                BlockDate {
                    epoch: 0,
                    slot_id: 0,
                },
            )
            .transaction(&sender, receiver.address(), output_value.into())
            .unwrap();
            sender.confirm_transaction();
            tx
        })
        .collect();

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        LinearFee::new(0, 0, 0),
        BlockDate::first().into(),
        FragmentSenderSetup::ignore_errors(),
    );

    let benchmark = benchmark_speed("test_expired_transactions_processing_speed")
        .target(Duration::from_secs(10))
        .start();

    let summary = fragment_sender
        .send_batch_fragments_in_chunks(transactions, 100, false, &jormungandr)
        .unwrap();

    assert!(summary.accepted.is_empty());
    assert_eq!(summary.rejected.len(), N_TRANSACTIONS);

    benchmark.stop().print();
}

#[test]
pub fn test_transactions_with_long_ttl_processing_speed() {
    const N_TRANSACTIONS: usize = 1_000;
    const MAX_EXPIRY_EPOCHS: u32 = 20;
    let mut sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_mempool(Mempool {
                pool_max_entries: N_TRANSACTIONS.into(),
                log_max_entries: N_TRANSACTIONS.into(),
                persistent_log: None,
            })
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap())
            .with_tx_max_expiry_epochs(MAX_EXPIRY_EPOCHS as u8),
    )
    .unwrap();

    time::wait_for_date(JLibBlockDate::new(0, 1), jormungandr.rest());

    let settings = jormungandr.rest().settings().unwrap();
    let block_date_generator = BlockDateGenerator::rolling(
        &settings,
        BlockDate {
            epoch: MAX_EXPIRY_EPOCHS + 1,
            slot_id: 0,
        },
        false,
    );

    let output_value = 1;

    let transactions: Vec<Fragment> = (0..N_TRANSACTIONS)
        .map(|_| {
            let tx = thor::FragmentBuilder::new(
                &jormungandr.genesis_block_hash(),
                &jormungandr.fees(),
                block_date_generator.block_date(),
            )
            .transaction(&sender, receiver.address(), output_value.into())
            .unwrap();
            sender.confirm_transaction();
            tx
        })
        .collect();

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        LinearFee::new(0, 0, 0),
        block_date_generator.block_date().into(),
        FragmentSenderSetup::ignore_errors(),
    );

    let benchmark = benchmark_speed("test_transactions_with_long_ttl_processing_speed")
        .target(Duration::from_secs(60))
        .start();

    let summary = fragment_sender
        .send_batch_fragments_in_chunks(transactions, 100, false, &jormungandr)
        .unwrap();

    assert!(summary.accepted.is_empty());
    assert_eq!(summary.rejected.len(), N_TRANSACTIONS);
    benchmark.stop().print();
}
