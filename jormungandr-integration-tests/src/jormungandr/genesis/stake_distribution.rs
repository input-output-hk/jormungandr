use crate::common::{
    jcli_wrapper, jormungandr::ConfigurationBuilder, startup, transaction_utils::TransactionHash,
};
use chain_impl_mockchain::fee::LinearFee;
use jormungandr_lib::{
    crypto::{account::Identifier as AccountIdentifier, hash::Hash},
    interfaces::{ActiveSlotCoefficient, Stake, StakeDistributionDto},
};
use std::str::FromStr;

#[test]
pub fn stake_distribution() {
    let mut sender = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();

    let stake_pool_owner_1 = startup::create_new_account_address();
    let fee = LinearFee::new(1, 1, 1);
    let (jormungandr, stake_pool_ids) = startup::start_stake_pool(
        &[stake_pool_owner_1.clone()],
        &[sender.clone(), receiver.clone()],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_rewards_history()
            .with_linear_fees(fee.clone())
            .with_slot_duration(3),
    )
    .unwrap();

    assert!(
        jormungandr.rest().stake_distribution_at(1).is_err(),
        "stake distribution for epoch in future should return error"
    );

    let transaction_fee: u64 = fee.constant + fee.coefficient * 2;
    let transaction_amount = 1_000;
    let initial_funds_per_account = 1_000_000_000;
    let stake_pool_id = Hash::from_str(stake_pool_ids.get(0).unwrap()).unwrap();

    assert_distribution(
        initial_funds_per_account * 2,
        0,
        (stake_pool_id, initial_funds_per_account),
        jormungandr.rest().stake_distribution().unwrap(),
    );

    let transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            stake_pool_owner_1.address(),
            transaction_amount.into(),
        )
        .unwrap()
        .encode();

    jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr);

    startup::sleep_till_next_epoch(10, &jormungandr.config);

    let identifier: AccountIdentifier = stake_pool_owner_1.identifier().into();
    let reward: u64 = jormungandr
        .rest()
        .epoch_reward_history(1)
        .unwrap()
        .accounts()
        .get(&identifier)
        .unwrap()
        .clone()
        .into();

    jcli_wrapper::assert_rest_account_get_stats(
        &stake_pool_owner_1.address().to_string(),
        &jormungandr.rest_address(),
    );

    startup::sleep_till_epoch(3,10, &jormungandr.config);

    jcli_wrapper::assert_rest_account_get_stats(
        &stake_pool_owner_1.address().to_string(),
        &jormungandr.rest_address(),
    );

    assert_distribution(
        initial_funds_per_account * 2 - transaction_fee - transaction_amount,
        0,
        (
            stake_pool_id.clone(),
            initial_funds_per_account + transaction_amount + reward,
        ),
        jormungandr.rest().stake_distribution_at(3).unwrap(),
    );
}

pub fn assert_distribution(
    unassigned: u64,
    dangling: u64,
    pool_stake: (Hash, u64),
    stake_distribution_dto: StakeDistributionDto,
) {
    let stake_distribution = stake_distribution_dto.stake;
    assert_eq!(
        Stake::from(unassigned),
        stake_distribution.unassigned,
        "unassigned"
    );
    assert_eq!(
        Stake::from(dangling),
        stake_distribution.dangling,
        "dangling"
    );
    let stake_pool_stake: Stake = *stake_distribution
        .pools
        .iter()
        .find(|(key, _)| *key == pool_stake.0)
        .map(|(_, stake)| stake)
        .unwrap();
    assert_eq!(
        Stake::from(pool_stake.1),
        stake_pool_stake,
        "stake pool {} stake",
        pool_stake.0
    );
}
