use crate::common::{jormungandr::ConfigurationBuilder, startup};
use chain_impl_mockchain::fee::LinearFee;
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, Mempool};
use jormungandr_testing_utils::{
    stake_pool::StakePool,
    testing::{FragmentGenerator, FragmentSender, FragmentSenderSetup},
};

#[test]
pub fn send_all_fragments() {
    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let fee = LinearFee::new(1, 1, 1);
    let value_to_send = 1;

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[receiver.clone()],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(3)
            .with_linear_fees(fee.clone())
            .with_explorer()
            .with_mempool(Mempool {
                pool_max_entries: 1_000_000usize.into(),
                log_max_entries: 1_000_000usize.into(),
            }),
    )
    .unwrap();

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::resend_3_times(),
    );
    let stake_pool = StakePool::new(&sender);
    fragment_sender.send_pool_registration(&sender, stake_pool.clone(), &jormungandr);

    let fragment_generator = FragmentGenerator::new(
        &mut sender,
        &receiver,
        vec![stake_pool],
        vec![],
        &jormungandr,
        jormungandr.explorer(),
        20,
    );
}
