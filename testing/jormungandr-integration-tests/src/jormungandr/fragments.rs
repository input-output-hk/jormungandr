use crate::common::{
    jcli::{FragmentsCheck, JCli},
    jormungandr::ConfigurationBuilder,
    startup,
};

use chain_impl_mockchain::{chaintypes::ConsensusType, fee::LinearFee};
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, BlockDate, Mempool};
use jormungandr_testing_utils::testing::{
    node::time, FragmentGenerator, FragmentSender, FragmentSenderSetup, FragmentVerifier,
    MemPoolCheck,
};
use jortestkit::prelude::Wait;
use std::time::Duration;

#[test]
pub fn send_all_fragments() {
    let receiver = startup::create_new_account_address();
    let sender = startup::create_new_account_address();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[receiver.clone()],
        ConfigurationBuilder::new()
            .with_block0_consensus(ConsensusType::GenesisPraos)
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(3)
            .with_linear_fees(LinearFee::new(1, 1, 1))
            .with_explorer()
            .with_mempool(Mempool {
                pool_max_entries: 1_000_000usize.into(),
                log_max_entries: 1_000_000usize.into(),
                persistent_log: None,
            }),
    )
    .unwrap();

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::no_verify(),
    );

    let time_era = jormungandr.time_era();

    let mut fragment_generator = FragmentGenerator::new(
        sender,
        receiver,
        jormungandr.to_remote(),
        time_era.slots_per_epoch(),
        30,
        30,
        fragment_sender,
    );

    fragment_generator.prepare(BlockDate::new(1, 0));

    let jcli: JCli = Default::default();

    let fragment_check = FragmentsCheck::new(jcli, &jormungandr);
    let wait = Wait::new(Duration::from_secs(1), 25);
    fragment_check.wait_until_all_processed(&wait).unwrap();

    time::wait_for_epoch(1, jormungandr.rest());

    let mem_checks: Vec<MemPoolCheck> = fragment_generator.send_all().unwrap();
    let verifier = FragmentVerifier;
    verifier
        .wait_and_verify_all_are_in_block(Duration::from_secs(2), mem_checks, &jormungandr)
        .unwrap();
}
