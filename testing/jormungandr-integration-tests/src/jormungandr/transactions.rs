use crate::common::{
    jcli::JCli, jormungandr::ConfigurationBuilder, startup, transaction_utils::TransactionHash,
};
use chain_impl_mockchain::fee::LinearFee;
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, Mempool, Value};

#[test]
pub fn accounts_funds_are_updated_after_transaction() {
    let jcli: JCli = Default::default();
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
            .with_linear_fees(fee)
            .with_mempool(Mempool {
                pool_max_entries: 1_000_000usize.into(),
                log_max_entries: 1_000_000usize.into(),
                persistent_log: None,
            }),
    )
    .unwrap();

    let sender_account_state_before = jcli
        .rest()
        .v0()
        .account_stats(sender.address().to_string(), jormungandr.rest_uri());
    let receiever_account_state_before = jcli
        .rest()
        .v0()
        .account_stats(&receiver.address().to_string(), &jormungandr.rest_uri());

    let sender_value_before = sender_account_state_before.value();
    let receiver_value_before = receiever_account_state_before.value();

    let new_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            receiver.address(),
            value_to_send.into(),
        )
        .unwrap()
        .encode();

    jcli.fragment_sender(&jormungandr)
        .send(&new_transaction)
        .assert_in_block();

    sender.confirm_transaction();

    let sender_account_state = jcli
        .rest()
        .v0()
        .account_stats(sender.address().to_string(), &jormungandr.rest_uri());
    let receiver_account_state = jcli
        .rest()
        .v0()
        .account_stats(receiver.address().to_string(), &jormungandr.rest_uri());

    let sender_value_before_u64: u64 = (*sender_value_before).into();
    let receiver_value_before_u64: u64 = (*receiver_value_before).into();

    let sender_last_reward: u64 = (*sender_account_state.last_rewards().reward()).into();

    let sender_expected_value: Value =
        (sender_value_before_u64 - value_to_send - fee.constant - (fee.coefficient * 2)
            + sender_last_reward)
            .into();
    let receiver_expected_value: Value = (receiver_value_before_u64 + value_to_send).into();

    let sender_account_state_value: Value = *sender_account_state.value();
    let receiver_account_state_value: Value = *receiver_account_state.value();

    assert_eq!(
        sender_expected_value, sender_account_state_value,
        "sender value after transaction"
    );
    assert_eq!(
        receiver_expected_value, receiver_account_state_value,
        "receiver value after transaction"
    );
}
